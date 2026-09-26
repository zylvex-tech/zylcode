//! Persisted provider scorecard — the measurement basis for Model Democracy.
//!
//! # Why this exists
//!
//! Model routing previously followed a fixed primary→fallback pair from
//! configuration. Which provider is actually *better* for this user's
//! workload was never measured: a provider that rate-limits every third
//! call keeps its configured priority forever.
//!
//! This module records per-provider dispatch outcomes (success/failure and
//! latency) in SQLite and ranks providers by Laplace-smoothed success rate.
//! The router consults the ranking when choosing which configured provider
//! to attempt first. Smoothed scoring keeps one bad sample from exiling a
//! provider, and thin data defers to the configured order — the system
//! never pretends to know more than it has measured.

use anyhow::Result;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

/// Laplace smoothing pseudo-count: with the default, a provider needs
/// several observed failures before its score drops below a fresh one.
const SMOOTHING_ALPHA: f64 = 1.0;
const SMOOTHING_BETA: f64 = 1.0;

/// Minimum samples before the scorecard's ranking overrides configuration.
pub const MIN_SAMPLES_FOR_RANKING: u64 = 5;

struct Inner {
    conn: Connection,
}

/// SQLite-backed scorecard. Cheap to clone via `Arc`.
pub struct ProviderScorecard {
    inner: Mutex<Inner>,
    db_path: PathBuf,
}

/// One provider's measured record.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderRecord {
    pub provider: String,
    pub successes: u64,
    pub failures: u64,
    /// Smoothed success estimate in [0, 1].
    pub score: f64,
    pub avg_latency_ms: Option<u64>,
}

impl ProviderScorecard {
    /// Open (creating if needed) the scorecard database at `path`.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS provider_outcomes (
                provider   TEXT NOT NULL,
                ok         INTEGER NOT NULL,
                latency_ms INTEGER NOT NULL,
                ts         TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_provider_outcomes_provider
                ON provider_outcomes(provider);",
        )?;
        Ok(Self {
            inner: Mutex::new(Inner { conn }),
            db_path: path.to_path_buf(),
        })
    }

    /// Record one dispatch outcome for `provider`.
    pub fn record(&self, provider: &str, ok: bool, latency_ms: u64) {
        if let Ok(inner) = self.inner.lock() {
            let _ = inner.conn.execute(
                "INSERT INTO provider_outcomes (provider, ok, latency_ms) VALUES (?1, ?2, ?3)",
                rusqlite::params![provider, ok as i64, latency_ms as i64],
            );
        }
        // Best-effort by contract: scorecard failures must never take down a
        // dispatch. Router callers invoke this fire-and-forget.
    }

    /// Laplace-smoothed score: (successes + α) / (total + α + β).
    pub fn score(successes: u64, failures: u64) -> f64 {
        let total = successes + failures;
        (successes as f64 + SMOOTHING_ALPHA) / (total as f64 + SMOOTHING_ALPHA + SMOOTHING_BETA)
    }

    /// Measured record for one provider (zeros when unmeasured).
    pub fn provider_record(&self, provider: &str) -> ProviderRecord {
        let inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => {
                return ProviderRecord {
                    provider: provider.to_string(),
                    successes: 0,
                    failures: 0,
                    score: Self::score(0, 0),
                    avg_latency_ms: None,
                };
            }
        };
        let mut stmt = match inner.conn.prepare(
            "SELECT SUM(ok), COUNT(*), AVG(latency_ms)
             FROM provider_outcomes WHERE provider = ?1",
        ) {
            Ok(s) => s,
            Err(_) => {
                return ProviderRecord {
                    provider: provider.to_string(),
                    successes: 0,
                    failures: 0,
                    score: Self::score(0, 0),
                    avg_latency_ms: None,
                };
            }
        };
        let row = stmt
            .query_row([provider], |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<f64>>(2)?,
                ))
            })
            .unwrap_or((None, 0, None));
        let total = row.1.max(0) as u64;
        // SUM(ok) counts successes; derive failures as total - successes.
        let successes = row.0.unwrap_or(0).max(0) as u64;
        ProviderRecord {
            provider: provider.to_string(),
            successes,
            failures: total.saturating_sub(successes),
            score: Self::score(successes, total.saturating_sub(successes)),
            avg_latency_ms: row.2.map(|ms| ms as u64),
        }
    }

    /// All measured providers, ranked by smoothed score (best first).
    /// Computed in a single locked pass (no nested locking — a re-entrant
    /// call here would deadlock the non-reentrant Mutex).
    /// Providers with fewer than [`MIN_SAMPLES_FOR_RANKING`] total samples
    /// are reported but excluded from ranking influence.
    pub fn ranking(&self) -> Vec<ProviderRecord> {
        let inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };
        let mut stmt = match inner.conn.prepare(
            "SELECT provider, SUM(ok), COUNT(*), AVG(latency_ms)
             FROM provider_outcomes
             GROUP BY provider",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = match stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<f64>>(3)?,
            ))
        }) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect::<Vec<_>>(),
            Err(_) => return Vec::new(),
        };
        let mut records: Vec<ProviderRecord> = rows
            .into_iter()
            .map(|(provider, successes_raw, total, avg_latency)| {
                let total = total.max(0) as u64;
                let successes = successes_raw.unwrap_or(0).max(0) as u64;
                ProviderRecord {
                    provider,
                    successes,
                    failures: total.saturating_sub(successes),
                    score: Self::score(successes, total.saturating_sub(successes)),
                    avg_latency_ms: avg_latency.map(|ms| ms as u64),
                }
            })
            .collect();
        records.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.successes.cmp(&a.successes))
        });
        records
    }

    /// The route order for `candidates` (already in configured fallback
    /// order): measured-best first once enough samples exist; the configured
    /// order otherwise. Returns the input unchanged when the scorecard has
    /// no qualifying data.
    pub fn route_order(&self, candidates: &[&str]) -> Vec<String> {
        let ranking = self.ranking();
        let qualifying: Vec<&ProviderRecord> = ranking
            .iter()
            .filter(|r| r.successes + r.failures >= MIN_SAMPLES_FOR_RANKING)
            .collect();
        if qualifying.is_empty() {
            return candidates.iter().map(|c| c.to_string()).collect();
        }
        // Stable sort: measured score desc, then configured order.
        let mut ordered: Vec<String> = candidates.iter().map(|c| c.to_string()).collect();
        ordered.sort_by_key(|name| {
            let rec = qualifying.iter().find(|r| &r.provider == name);
            let score = rec.map(|r| r.score).unwrap_or(0.0);
            std::cmp::Reverse((score * 1_000_000.0) as i64)
        });
        ordered
    }

    /// Database path (for diagnostics).
    pub fn path(&self) -> &Path {
        &self.db_path
    }
}

/// Wall-clock helper for latency capture at dispatch sites.
pub struct LatencyGuard {
    start: Instant,
}

impl LatencyGuard {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_scorecard() -> (tempfile::TempDir, ProviderScorecard) {
        let dir = tempfile::tempdir().unwrap();
        let sc = ProviderScorecard::open(&dir.path().join("scorecard.db")).unwrap();
        (dir, sc)
    }

    #[test]
    fn a_fresh_scorecard_defers_to_configuration() {
        let (_dir, sc) = temp_scorecard();
        let order = sc.route_order(&["anthropic", "ollama", "openrouter"]);
        assert_eq!(order, vec!["anthropic", "ollama", "openrouter"]);
    }

    #[test]
    fn repeated_failures_demote_a_provider() {
        let (_dir, sc) = temp_scorecard();
        // anthropic fails 6 times; ollama succeeds 6 times.
        for _ in 0..6 {
            sc.record("anthropic", false, 100);
            sc.record("ollama", true, 40);
        }
        let order = sc.route_order(&["anthropic", "ollama"]);
        assert_eq!(order.first().unwrap(), "ollama", "{order:?}");
        let rec = sc.provider_record("anthropic");
        assert_eq!(rec.failures, 6);
        assert!(
            rec.score < 0.5,
            "smoothed score must drop below 0.5: {}",
            rec.score
        );
    }

    #[test]
    fn thin_data_does_not_overthrow_configuration() {
        let (_dir, sc) = temp_scorecard();
        // Only 2 samples: below MIN_SAMPLES_FOR_RANKING.
        sc.record("openrouter", true, 10);
        sc.record("openrouter", true, 10);
        let order = sc.route_order(&["anthropic", "openrouter"]);
        assert_eq!(order.first().unwrap(), "anthropic");
    }

    #[test]
    fn one_failure_does_not_exile_a_healthy_provider() {
        let (_dir, sc) = temp_scorecard();
        for _ in 0..9 {
            sc.record("anthropic", true, 50);
        }
        sc.record("anthropic", false, 50);
        let rec = sc.provider_record("anthropic");
        // (9+1)/(10+2) ≈ 0.83 — still comfortably above a 50% provider.
        assert!(rec.score > 0.8, "{}", rec.score);
    }

    #[test]
    fn outcomes_persist_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scorecard.db");
        {
            let sc = ProviderScorecard::open(&path).unwrap();
            sc.record("ollama", true, 30);
        }
        let reopened = ProviderScorecard::open(&path).unwrap();
        let rec = reopened.provider_record("ollama");
        assert_eq!(rec.successes, 1, "persistence required for durable memory");
    }
}
