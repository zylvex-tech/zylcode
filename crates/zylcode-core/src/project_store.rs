//! Persistent Project System store (wave 2 — foundation).
//!
//! # Why this exists
//!
//! The Project System had types ([`crate::project::Project`]) and real
//! filesystem operations ([`crate::project::ProjectFilesystem`]) but no
//! *persistence*: a project record lived only in UI state and vanished on
//! restart. This module is the store that makes a project durable state:
//!
//! - a defined, versioned schema (`schema_version`);
//! - forward migration with recorded steps (never silently invented);
//! - atomic writes (temp + rename) matching the missions/ledger convention;
//! - recovery: a corrupt or future-version store degrades to an explicit
//!   [`ProjectStoreState`], not a panic and not silent data loss;
//! - import/export as portable JSON.
//!
//! # Persistence convention
//!
//! `PROJECTS_PATH` env override → `<root>/.zylcode/projects.json`. The
//! `.zylcode/` directory is already the established persistence root for
//! missions, the ledger, the vector cache, and the provider scorecard.
//!
//! # Schema history
//!
//! - v1: initial envelope `{ schema_version, projects[] }` with `id`, `name`,
//!   `root_path`, optional `repository_remote`/`notes`, timestamps.
//!   (A bare, pre-envelope `schema_version: 0` document migrates by stamping.)
//! - v2: adds `decisions[]` — the inspectable history PROJECT_SYSTEM.md §7.2
//!   requires ("every mutation is recorded as a Project Decision entry").
//!   v1 records migrate by gaining a `schema_migrated` decision, so the change
//!   is visible in the project's own history rather than only in a log.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Current on-disk schema version. Bump only with a migration step below.
pub const PROJECTS_SCHEMA_VERSION: u32 = 2;

/// One entry in a project's inspectable history.
///
/// PROJECT_SYSTEM.md §7.2: "Every mutation is recorded as a Project Decision
/// entry so the project's history is inspectable." Decisions are append-only
/// and carry their own timestamp; they are not a replacement for the evidence
/// ledger, which records engineering proof rather than store mutations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectDecision {
    /// ISO 8601 timestamp of the mutation.
    pub at: String,
    /// Machine-readable kind: `created`, `updated`, `imported`, `schema_migrated`.
    pub kind: String,
    /// Human-readable summary, including what changed.
    pub summary: String,
}

/// A durable project record. Deliberately distinct from
/// [`crate::project::Project`], which carries runtime/UI state: this is the
/// persisted identity. Structural fields (id, root_path, created_at) are
/// immutable after registration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectRecord {
    /// Unique project identifier (UUID v4).
    pub id: String,
    /// Human-readable project name (non-empty).
    pub name: String,
    /// Absolute path to the project root directory.
    pub root_path: String,
    /// Optional git remote URL for repository association.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_remote: Option<String>,
    /// Free-form operator notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last-accessed timestamp, updated on open/touch.
    pub last_accessed: String,
    /// Schema version this record was written under.
    pub schema_version: u32,
    /// Append-only mutation history (added in schema v2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<ProjectDecision>,
}

impl ProjectRecord {
    /// Create a record with a fresh id and timestamps set to now.
    pub fn new(name: &str, root_path: &Path) -> Result<Self> {
        let name = name.trim();
        anyhow::ensure!(!name.is_empty(), "project name must not be empty");
        anyhow::ensure!(
            root_path.is_absolute(),
            "project root_path must be absolute: {}",
            root_path.display()
        );
        let now = chrono::Utc::now().to_rfc3339();
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            root_path: root_path
                .to_str()
                .context("project root_path is not valid UTF-8")?
                .to_string(),
            repository_remote: None,
            notes: None,
            created_at: now.clone(),
            last_accessed: now,
            schema_version: PROJECTS_SCHEMA_VERSION,
            decisions: Vec::new(),
        })
    }
}

/// What the store found on disk. `Ready` is the only state that permits
/// mutation; everything else is an explicit, visible condition.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectStoreState {
    /// Store loaded cleanly at the current schema version.
    Ready,
    /// Store loaded after migrating from an older schema (from → to).
    Migrated { from: u32, to: u32 },
    /// No store file yet — first open or a fresh workspace.
    Empty,
    /// Store file exists but is corrupt. The raw bytes are preserved on disk
    /// (never truncated) and the store refuses mutation until repaired.
    Corrupt { detail: String },
    /// Store file was written by a *newer* schema version. Refuse to guess.
    FutureVersion { found: u32, supported: u32 },
}

/// The raw on-disk document: a version envelope around the records.
#[derive(Debug, Serialize, Deserialize)]
struct ProjectsDocument {
    schema_version: u32,
    #[serde(default)]
    projects: Vec<ProjectRecord>,
}

/// File-backed project store. All mutation is serialized by an internal
/// mutex; `state` is interior-mutable so read paths can report what they saw.
///
/// The internal mutex only serializes threads *in this process*. Two
/// processes — the CLI and the desktop, which both open the same
/// `.zylcode/projects.json` — would otherwise interleave read-modify-write
/// cycles and lose a decision or a record. `_file_lock` is an OS advisory lock
/// on `<store>.lock`, held for the store's whole lifetime, so the second
/// process fails loudly with a nameable error instead of silently clobbering
/// the first. The OS releases it if the holder dies, which is why this is a
/// file lock and not a PID file.
#[derive(Debug)]
pub struct ProjectStore {
    path: PathBuf,
    lock: Mutex<()>,
    state: Mutex<ProjectStoreState>,
    _file_lock: fs::File,
}

/// Where project state is persisted (env override → `<root>/.zylcode/projects.json`).
pub fn projects_path(root: &Path) -> PathBuf {
    if let Ok(p) = std::env::var("PROJECTS_PATH") {
        return PathBuf::from(p);
    }
    root.join(".zylcode").join("projects.json")
}

/// How long `open` waits for another process to release the store before it
/// reports the contention. A store open is fast; two seconds of waiting covers
/// a CLI command finishing while the desktop starts, and anything longer means
/// a human should look.
const STORE_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const STORE_LOCK_POLL: std::time::Duration = std::time::Duration::from_millis(20);

/// `<store>.lock` beside the store file. Never deleted: removing a lock file
/// while another process holds it reintroduces the race the lock exists to
/// prevent.
fn store_lock_path(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".lock");
    PathBuf::from(os)
}

/// Take the cross-process advisory lock, or fail with an error an operator can
/// act on. The lock is released when the returned file is dropped, and by the
/// OS if the holder dies.
fn acquire_store_lock(path: &Path) -> Result<fs::File> {
    let lock_path = store_lock_path(path);
    let file = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .with_context(|| format!("failed to open lock file {}", lock_path.display()))?;
    let deadline = std::time::Instant::now() + STORE_LOCK_TIMEOUT;
    loop {
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => return Ok(file),
            Err(err) if is_lock_contention(&err) => {
                if std::time::Instant::now() >= deadline {
                    anyhow::bail!(
                        "project store is in use by another process: {} (waited {}s). \
                         Close the other ZylCode surface and retry; nothing was modified.",
                        path.display(),
                        STORE_LOCK_TIMEOUT.as_secs()
                    );
                }
                std::thread::sleep(STORE_LOCK_POLL);
            }
            Err(err) => {
                return Err(err).with_context(|| format!("failed to lock {}", lock_path.display()))
            }
        }
    }
}

/// Windows reports a held lock as `ERROR_LOCK_VIOLATION` (33) rather than the
/// portable `WouldBlock`, so both are treated as contention.
fn is_lock_contention(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::WouldBlock || err.raw_os_error() == Some(33)
}

/// Strip the Windows verbatim (`\\?\`) prefix `std::fs::canonicalize` adds.
/// Users register `C:\Projects\foo`, not `\\?\C:\Projects\foo`; UNC paths
/// (`\\?\UNC\server\share`) normalize to `\\server\share`. Other platforms
/// are returned unchanged.
fn normalize_absolute(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return PathBuf::from(rest);
    }
    p.to_path_buf()
}

/// Canonical read: parse the document and classify what was found. A corrupt
/// file returns an error *after* reporting `Corrupt` in the state; the caller
/// decides whether that is fatal (open) or transient (list after repair).
fn read_document(path: &Path) -> (Result<Vec<ProjectRecord>>, ProjectStoreState) {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return (Ok(Vec::new()), ProjectStoreState::Empty);
        }
        Err(e) => {
            let detail = e.to_string();
            return (
                Err(e).with_context(|| format!("failed to read {}", path.display())),
                ProjectStoreState::Corrupt {
                    detail: format!("unreadable: {detail}"),
                },
            );
        }
    };
    if raw.trim().is_empty() {
        return (Ok(Vec::new()), ProjectStoreState::Empty);
    }
    let doc: ProjectsDocument = match serde_json::from_str(&raw) {
        Ok(d) => d,
        Err(e) => {
            return (
                Err(anyhow::anyhow!(
                    "project store at {} is corrupt: {e}; file preserved for manual repair",
                    path.display()
                )),
                ProjectStoreState::Corrupt {
                    detail: e.to_string(),
                },
            );
        }
    };
    if doc.schema_version > PROJECTS_SCHEMA_VERSION {
        return (
            Err(anyhow::anyhow!(
                "project store schema v{} is newer than supported v{}",
                doc.schema_version,
                PROJECTS_SCHEMA_VERSION
            )),
            ProjectStoreState::FutureVersion {
                found: doc.schema_version,
                supported: PROJECTS_SCHEMA_VERSION,
            },
        );
    }
    if doc.schema_version < PROJECTS_SCHEMA_VERSION {
        let mut projects = doc.projects;
        // Forward-only stamping. Each migrated record gains a visible decision
        // so the upgrade appears in that project's own history, not only in a
        // developer log (v1→v2 adds `decisions` itself).
        for p in &mut projects {
            if p.schema_version < PROJECTS_SCHEMA_VERSION {
                let from = p.schema_version;
                p.schema_version = PROJECTS_SCHEMA_VERSION;
                p.decisions.push(ProjectDecision {
                    at: chrono::Utc::now().to_rfc3339(),
                    kind: "schema_migrated".to_string(),
                    summary: format!(
                        "project store migrated from schema v{from} to v{PROJECTS_SCHEMA_VERSION}"
                    ),
                });
            }
        }
        let state = ProjectStoreState::Migrated {
            from: doc.schema_version,
            to: PROJECTS_SCHEMA_VERSION,
        };
        return (Ok(projects), state);
    }
    (Ok(doc.projects), ProjectStoreState::Ready)
}

impl ProjectStore {
    /// Open (or initialize) the store at its path. Fails hard on
    /// corrupt/future stores — the operator must see and repair those — and on
    /// a store another process is holding.
    pub fn open(root: &Path) -> Result<Self> {
        let path = projects_path(root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let file_lock = acquire_store_lock(&path)?;
        let (records, state) = read_document(&path);
        let records = records?;
        let store = Self {
            path,
            lock: Mutex::new(()),
            state: Mutex::new(state),
            _file_lock: file_lock,
        };
        if matches!(
            store.current_state(),
            ProjectStoreState::Migrated { .. } | ProjectStoreState::Empty
        ) && !records.is_empty()
        {
            // Persist migrated content so the migration is durable and the
            // file's envelope always matches what we can read.
            store.write(&records)?;
            store.set_state(ProjectStoreState::Ready);
        }
        Ok(store)
    }

    /// The store's current state as of the last disk observation.
    pub fn state(&self) -> ProjectStoreState {
        self.current_state()
    }

    fn current_state(&self) -> ProjectStoreState {
        self.state.lock().unwrap().clone()
    }

    fn set_state(&self, s: ProjectStoreState) {
        *self.state.lock().unwrap() = s;
    }

    fn records(&self) -> Result<Vec<ProjectRecord>> {
        let (records, state) = read_document(&self.path);
        self.set_state(state);
        records
    }

    fn write(&self, projects: &[ProjectRecord]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let doc = ProjectsDocument {
            schema_version: PROJECTS_SCHEMA_VERSION,
            projects: projects.to_vec(),
        };
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&doc)?)
            .with_context(|| format!("failed to write {}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("failed to persist {}", self.path.display()))?;
        Ok(())
    }

    /// Register a new project. Name and absolute root are validated; root
    /// must exist and be a directory. Duplicate roots are rejected — one
    /// directory is one project.
    pub fn register(&self, name: &str, root_path: &Path) -> Result<ProjectRecord> {
        anyhow::ensure!(
            root_path.is_dir(),
            "project root does not exist or is not a directory: {}",
            root_path.display()
        );
        let _g = self.lock.lock().unwrap();
        let mut projects = self.records()?;
        let canonical = normalize_absolute(
            &root_path
                .canonicalize()
                .unwrap_or_else(|_| root_path.to_path_buf()),
        );
        if projects.iter().any(|p| {
            normalize_absolute(
                &Path::new(&p.root_path)
                    .canonicalize()
                    .unwrap_or_else(|_| PathBuf::from(&p.root_path)),
            ) == canonical
        }) {
            anyhow::bail!(
                "a project is already registered for {}",
                canonical.display()
            );
        }
        let mut record = ProjectRecord::new(name, &canonical)?;
        record.decisions.push(ProjectDecision {
            at: record.created_at.clone(),
            kind: "created".to_string(),
            summary: format!("project registered at {}", record.root_path),
        });
        projects.push(record.clone());
        self.write(&projects)?;
        self.set_state(ProjectStoreState::Ready);
        Ok(record)
    }

    /// All project records, most recently accessed first.
    pub fn list(&self) -> Result<Vec<ProjectRecord>> {
        let mut projects = self.records()?;
        projects.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        Ok(projects)
    }

    /// Fetch one project by id.
    pub fn get(&self, id: &str) -> Result<Option<ProjectRecord>> {
        Ok(self.records()?.into_iter().find(|p| p.id == id))
    }

    /// Mark a project accessed (updates last_accessed, bumps it to the top).
    pub fn touch(&self, id: &str) -> Result<()> {
        let _g = self.lock.lock().unwrap();
        let mut projects = self.records()?;
        let now = chrono::Utc::now().to_rfc3339();
        let mut found = false;
        for p in &mut projects {
            if p.id == id {
                p.last_accessed = now.clone();
                found = true;
            }
        }
        anyhow::ensure!(found, "unknown project id: {id}");
        self.write(&projects)
    }

    /// Update mutable fields (name/notes/repository_remote). Structural
    /// fields (id, root_path, created_at) are immutable by design —
    /// re-register instead.
    pub fn update(
        &self,
        id: &str,
        name: Option<String>,
        notes: Option<String>,
        repository_remote: Option<String>,
    ) -> Result<ProjectRecord> {
        let _g = self.lock.lock().unwrap();
        let mut projects = self.records()?;
        let mut updated: Option<ProjectRecord> = None;
        for p in &mut projects {
            if p.id == id {
                let mut changed: Vec<&str> = Vec::new();
                if let Some(n) = name.as_deref() {
                    let n = n.trim().to_string();
                    anyhow::ensure!(!n.is_empty(), "project name must not be empty");
                    if n != p.name {
                        p.name = n;
                        changed.push("name");
                    }
                }
                if let Some(notes) = notes.as_deref() {
                    let next = if notes.trim().is_empty() {
                        None
                    } else {
                        Some(notes.to_string())
                    };
                    if next != p.notes {
                        p.notes = next;
                        changed.push("notes");
                    }
                }
                if let Some(r) = repository_remote.as_deref() {
                    let next = if r.trim().is_empty() {
                        None
                    } else {
                        Some(r.to_string())
                    };
                    if next != p.repository_remote {
                        p.repository_remote = next;
                        changed.push("repository_remote");
                    }
                }
                let now = chrono::Utc::now().to_rfc3339();
                p.last_accessed = now.clone();
                if !changed.is_empty() {
                    p.decisions.push(ProjectDecision {
                        at: now,
                        kind: "updated".to_string(),
                        summary: format!("updated {}", changed.join(", ")),
                    });
                }
                updated = Some(p.clone());
            }
        }
        let updated = updated.context(format!("unknown project id: {id}"))?;
        self.write(&projects)?;
        Ok(updated)
    }

    /// Remove a project record. The workspace directory is never touched —
    /// this removes the association, not the files.
    pub fn remove(&self, id: &str) -> Result<()> {
        let _g = self.lock.lock().unwrap();
        let mut projects = self.records()?;
        let before = projects.len();
        projects.retain(|p| p.id != id);
        anyhow::ensure!(projects.len() < before, "unknown project id: {id}");
        self.write(&projects)
    }

    /// Export all records as portable JSON (import/export boundary).
    pub fn export_json(&self) -> Result<String> {
        let doc = ProjectsDocument {
            schema_version: PROJECTS_SCHEMA_VERSION,
            projects: self.records()?,
        };
        Ok(serde_json::to_string_pretty(&doc)?)
    }

    /// Import records from exported JSON. Existing records with the same id
    /// or root_path are left untouched; new ones are appended. Returns how
    /// many records were added.
    pub fn import_json(&self, json: &str) -> Result<usize> {
        Ok(self.import_json_report(json)?.added)
    }

    /// Import with the explicit report PROJECT_SYSTEM.md R3 requires.
    ///
    /// A project may legitimately be imported on a machine where its repository
    /// is not cloned yet, so an unresolved root is **not** an error: the record
    /// keeps its identity and the path is reported to the caller (and recorded
    /// in the project's decision history) rather than being silently dropped.
    pub fn import_json_report(&self, json: &str) -> Result<ImportReport> {
        let doc: ProjectsDocument = serde_json::from_str(json)
            .context("import payload is not a valid projects document")?;
        anyhow::ensure!(
            doc.schema_version <= PROJECTS_SCHEMA_VERSION,
            "import payload schema v{} newer than supported v{}",
            doc.schema_version,
            PROJECTS_SCHEMA_VERSION
        );
        let _g = self.lock.lock().unwrap();
        let mut projects = self.records()?;
        let mut report = ImportReport::default();
        for p in doc.projects {
            let dup_id = projects.iter().any(|e| e.id == p.id);
            let dup_root = projects.iter().any(|e| e.root_path == p.root_path);
            if dup_id || dup_root {
                report.skipped_existing += 1;
                continue;
            }
            let mut p = p;
            p.schema_version = PROJECTS_SCHEMA_VERSION;
            let root_exists = Path::new(&p.root_path).is_dir();
            p.decisions.push(ProjectDecision {
                at: chrono::Utc::now().to_rfc3339(),
                kind: "imported".to_string(),
                summary: if root_exists {
                    format!("imported; root resolved at {}", p.root_path)
                } else {
                    format!("imported; root UNRESOLVED on this machine: {}", p.root_path)
                },
            });
            if !root_exists {
                report.unresolved_roots.push(p.root_path.clone());
            }
            projects.push(p);
            report.added += 1;
        }
        self.write(&projects)?;
        Ok(report)
    }
}

/// Outcome of an import, including the references that did not resolve here.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportReport {
    /// Records newly added to this store.
    pub added: usize,
    /// Records skipped because their id or root_path was already present.
    pub skipped_existing: usize,
    /// Imported project roots that do not exist on this machine.
    pub unresolved_roots: Vec<String>,
}
#[cfg(test)]
mod tests {
    use super::*;

    /// `PROJECTS_PATH` is process-global, so tests that set it must not run
    /// concurrently — one rogue interleaving points another test's store at
    /// the wrong file. Serialise every env-sensitive test on this lock.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Set up an isolated store path; returns the guard + store directory.
    /// Workspaces are created *by each test* so they live for the test body.
    fn isolated() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
        let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var(
            "PROJECTS_PATH",
            dir.path().join(".zylcode/projects-test.json"),
        );
        (guard, dir)
    }

    #[test]
    fn register_persists_and_survives_reopen() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let ws = tempfile::tempdir().unwrap();
        std::env::set_var(
            "PROJECTS_PATH",
            dir.path().join(".zylcode/projects-test.json"),
        );
        let store = ProjectStore::open(dir.path()).unwrap();
        assert!(matches!(store.state(), ProjectStoreState::Empty));
        let rec = store.register("alpha", ws.path()).unwrap();
        assert_eq!(rec.schema_version, PROJECTS_SCHEMA_VERSION);

        // Reopen: the record must survive restart. This is the property the
        // Project System previously lacked.
        drop(store);
        let reopened = ProjectStore::open(dir.path()).unwrap();
        assert!(matches!(reopened.state(), ProjectStoreState::Ready));
        let all = reopened.list().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, rec.id);
        assert_eq!(all[0].name, "alpha");
        // The store may resolve aliases (Windows 8.3 short names, verbatim
        // prefixes); the invariant is that it names the *same real directory*.
        assert_eq!(
            Path::new(&all[0].root_path).canonicalize().unwrap(),
            ws.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn register_rejects_missing_root_and_duplicate() {
        let (_g, dir) = isolated();
        let store = ProjectStore::open(dir.path()).unwrap();
        assert!(store
            .register("ghost", Path::new("Z:/definitely/not/here"))
            .is_err());
        let ws = tempfile::tempdir().unwrap();
        store.register("one", ws.path()).unwrap();
        assert!(
            store.register("two", ws.path()).is_err(),
            "duplicate root must be rejected"
        );
    }

    #[test]
    fn corrupt_store_is_preserved_and_reported() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("corrupt.json");
        std::fs::write(&path, "{not json at all").unwrap();
        std::env::set_var("PROJECTS_PATH", &path);

        let store = ProjectStore::open(dir.path());
        assert!(store.is_err(), "corrupt store must not open silently");
        // The raw bytes are still on disk for manual repair.
        let raw = std::fs::read_to_string(&path).unwrap();
        assert_eq!(raw, "{not json at all");
    }

    #[test]
    fn future_version_is_refused_not_guessed() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("future.json");
        let doc = serde_json::json!({
            "schema_version": PROJECTS_SCHEMA_VERSION + 1,
            "projects": []
        });
        std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
        std::env::set_var("PROJECTS_PATH", &path);

        assert!(
            ProjectStore::open(dir.path()).is_err(),
            "future schema must be refused"
        );
    }

    #[test]
    fn migration_stamps_legacy_records() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        let legacy = serde_json::json!({
            "schema_version": 0,
            "projects": [{
                "id": "legacy-1",
                "name": "legacy",
                "root_path": "/tmp/legacy",
                "created_at": "2026-01-01T00:00:00Z",
                "last_accessed": "2026-01-01T00:00:00Z",
                "schema_version": 0
            }]
        });
        std::fs::write(&path, serde_json::to_string(&legacy).unwrap()).unwrap();
        std::env::set_var("PROJECTS_PATH", &path);

        let store = ProjectStore::open(dir.path()).unwrap();
        assert!(
            matches!(store.state(), ProjectStoreState::Ready),
            "post-migration store should persist as Ready, got {:?}",
            store.state()
        );
        let all = store.list().unwrap();
        assert_eq!(all[0].schema_version, PROJECTS_SCHEMA_VERSION);
        // Migration persisted: reopening reads Ready, not Migrated again.
        drop(store);
        let reopened = ProjectStore::open(dir.path()).unwrap();
        assert!(matches!(reopened.state(), ProjectStoreState::Ready));
        assert_eq!(reopened.list().unwrap()[0].id, "legacy-1");
    }

    #[test]
    fn update_never_touches_structural_fields() {
        let (_g, dir) = isolated();
        let store = ProjectStore::open(dir.path()).unwrap();
        let ws = tempfile::tempdir().unwrap();
        let rec = store.register("before", ws.path()).unwrap();
        let updated = store
            .update(
                &rec.id,
                Some("after".into()),
                Some("notes here".into()),
                Some("https://github.com/zylvex-tech/zylcode".into()),
            )
            .unwrap();
        assert_eq!(updated.name, "after");
        assert_eq!(updated.id, rec.id, "id immutable");
        assert_eq!(updated.root_path, rec.root_path, "root immutable");
        assert_eq!(updated.created_at, rec.created_at, "created_at immutable");
        assert_eq!(
            updated.repository_remote.as_deref(),
            Some("https://github.com/zylvex-tech/zylcode")
        );
    }

    #[test]
    fn remove_drops_record_but_not_files() {
        let (_g, dir) = isolated();
        let store = ProjectStore::open(dir.path()).unwrap();
        let ws = tempfile::tempdir().unwrap();
        let marker = ws.path().join("keep.txt");
        std::fs::write(&marker, "data").unwrap();
        let rec = store.register("doomed", ws.path()).unwrap();
        store.remove(&rec.id).unwrap();
        assert!(store.get(&rec.id).unwrap().is_none());
        assert!(marker.exists(), "workspace files must survive removal");
    }

    #[test]
    fn export_import_roundtrip_and_dedup() {
        let (_g, dir) = isolated();
        let store = ProjectStore::open(dir.path()).unwrap();
        let ws = tempfile::tempdir().unwrap();
        store.register("roundtrip", ws.path()).unwrap();
        let json = store.export_json().unwrap();

        // Import into the *same* store: dedup by id/root must add nothing.
        assert_eq!(store.import_json(&json).unwrap(), 0);

        // Import into a fresh store: adds exactly one.
        let dir2 = tempfile::tempdir().unwrap();
        std::env::set_var("PROJECTS_PATH", dir2.path().join("imported.json"));
        let store2 = ProjectStore::open(dir2.path()).unwrap();
        assert_eq!(store2.import_json(&json).unwrap(), 1);
        assert_eq!(store2.list().unwrap().len(), 1);
    }

    #[test]
    fn mutations_are_recorded_as_decisions() {
        let (_g, dir) = isolated();
        let store = ProjectStore::open(dir.path()).unwrap();
        let ws = tempfile::tempdir().unwrap();
        let rec = store.register("history", ws.path()).unwrap();
        assert_eq!(rec.decisions.len(), 1, "registration must be recorded");
        assert_eq!(rec.decisions[0].kind, "created");

        let updated = store
            .update(&rec.id, Some("history-2".into()), None, None)
            .unwrap();
        assert_eq!(updated.decisions.len(), 2);
        assert_eq!(updated.decisions[1].kind, "updated");
        assert!(
            updated.decisions[1].summary.contains("name"),
            "decision must say what changed: {:?}",
            updated.decisions[1]
        );

        // A no-op update must not manufacture history.
        let unchanged = store
            .update(&rec.id, Some("history-2".into()), None, None)
            .unwrap();
        assert_eq!(unchanged.decisions.len(), 2);

        // History is durable.
        drop(store);
        let reopened = ProjectStore::open(dir.path()).unwrap();
        assert_eq!(reopened.get(&rec.id).unwrap().unwrap().decisions.len(), 2);
    }

    #[test]
    fn import_reports_unresolved_roots_instead_of_dropping_them() {
        let (_g, dir) = isolated();
        let store = ProjectStore::open(dir.path()).unwrap();
        let missing = dir.path().join("not-cloned-yet");
        let payload = serde_json::json!({
            "schema_version": PROJECTS_SCHEMA_VERSION,
            "projects": [{
                "id": "portable-1",
                "name": "portable",
                "root_path": missing.to_string_lossy(),
                "created_at": "2026-01-01T00:00:00Z",
                "last_accessed": "2026-01-01T00:00:00Z",
                "schema_version": PROJECTS_SCHEMA_VERSION
            }]
        });

        let report = store
            .import_json_report(&serde_json::to_string(&payload).unwrap())
            .unwrap();
        assert_eq!(report.added, 1, "identity must survive a missing root");
        assert_eq!(report.skipped_existing, 0);
        assert_eq!(report.unresolved_roots.len(), 1);
        assert_eq!(Path::new(&report.unresolved_roots[0]), missing);

        let imported = store.get("portable-1").unwrap().unwrap();
        assert!(
            imported
                .decisions
                .iter()
                .any(|d| d.kind == "imported" && d.summary.contains("UNRESOLVED")),
            "unresolved reference must appear in project history: {:?}",
            imported.decisions
        );

        // Re-import must report the skip, not add a duplicate.
        let again = store
            .import_json_report(&serde_json::to_string(&payload).unwrap())
            .unwrap();
        assert_eq!(again.added, 0);
        assert_eq!(again.skipped_existing, 1);
    }

    #[test]
    fn v1_document_migrates_to_v2_with_visible_history() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.json");
        let v1 = serde_json::json!({
            "schema_version": 1,
            "projects": [{
                "id": "v1-project",
                "name": "v1",
                "root_path": "/tmp/v1",
                "created_at": "2026-01-01T00:00:00Z",
                "last_accessed": "2026-01-01T00:00:00Z",
                "schema_version": 1
            }]
        });
        std::fs::write(&path, serde_json::to_string(&v1).unwrap()).unwrap();
        std::env::set_var("PROJECTS_PATH", &path);

        let store = ProjectStore::open(dir.path()).unwrap();
        let migrated = store.get("v1-project").unwrap().unwrap();
        assert_eq!(migrated.schema_version, PROJECTS_SCHEMA_VERSION);
        assert_eq!(migrated.decisions.len(), 1);
        assert_eq!(migrated.decisions[0].kind, "schema_migrated");
        assert!(migrated.decisions[0].summary.contains("v1 to v2"));
    }

    #[test]
    fn touch_updates_ordering() {
        let (_g, dir) = isolated();
        let store = ProjectStore::open(dir.path()).unwrap();
        let ws = tempfile::tempdir().unwrap();
        let ws_b = tempfile::tempdir().unwrap();
        let a = store.register("a", ws.path()).unwrap();
        let b = store.register("b", ws_b.path()).unwrap();
        store.touch(&a.id).unwrap();
        let listed = store.list().unwrap();
        assert_eq!(listed[0].id, a.id);
        assert_eq!(listed[1].id, b.id);
    }

    /// Two surfaces (CLI and desktop) open the same store. Without an OS lock
    /// the second read-modify-write cycle silently eats the first's record; the
    /// store refuses instead, and says so.
    #[test]
    fn a_second_process_holding_the_store_is_refused_then_released() {
        let (_g, dir) = isolated();
        let first = ProjectStore::open(dir.path()).expect("first open");

        let err = ProjectStore::open(dir.path()).expect_err("second open must be refused");
        let message = format!("{err:#}");
        assert!(
            message.contains("in use by another process"),
            "error must name the contention, got: {message}"
        );

        drop(first);
        ProjectStore::open(dir.path()).expect("lock is released with the store");
    }

    /// A lock file left behind by a dead holder must not wedge the store: the
    /// OS releases advisory locks on process exit, so a fresh open succeeds.
    #[test]
    fn a_stale_lock_file_does_not_block_a_fresh_open() {
        let (_g, dir) = isolated();
        let lock = store_lock_path(&projects_path(dir.path()));
        std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
        std::fs::write(&lock, b"leftover from a killed process").unwrap();

        ProjectStore::open(dir.path()).expect("stale lock file must not block opening");
    }
}
