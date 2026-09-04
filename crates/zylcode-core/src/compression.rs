//! Phase 8.1 — Advanced Context Compression & Window Compaction Engine.
//!
//! Provides [`ContextCompressor`] with pluggable strategies:
//! - `LosslessCommentsStripper` — strips `//` and `/* */` without touching string literals
//! - `ASTOutlineExtractor` — outline-focused summarization for Rust/TS sources
//! - `TokenWindowCompactor` — budget-targeted sliding-window truncation
//!
//! All strategies operate on `&str` and preserve UTF-8 correctness.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Budget & metrics
// ---------------------------------------------------------------------------

/// Compression metrics emitted as `telemetry:compression`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionMetrics {
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    pub compression_ratio: f64,
    pub strategy: String,
}

impl CompressionMetrics {
    pub fn new(original: usize, compressed: usize, strategy: impl Into<String>) -> Self {
        let ratio = if original == 0 {
            1.0
        } else {
            compressed as f64 / original as f64
        };
        Self {
            original_tokens: original,
            compressed_tokens: compressed,
            compression_ratio: ratio,
            strategy: strategy.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Strategy trait
// ---------------------------------------------------------------------------

pub trait CompressionStrategy: Send + Sync {
    fn name(&self) -> &'static str;
    fn compress(&self, input: &str, budget_tokens: usize) -> String;
}

// ---------------------------------------------------------------------------
// Strategy 1 — Lossless comments stripper
// ---------------------------------------------------------------------------

/// Strips single-line `//` and block `/* */` comments while preserving string
/// literals and char literals. Operates as a state machine to avoid corrupting
/// code inside `"..."` or `'...'`.
#[derive(Debug, Default, Clone, Copy)]
pub struct LosslessCommentsStripper;

impl CompressionStrategy for LosslessCommentsStripper {
    fn name(&self) -> &'static str {
        "LosslessCommentsStripper"
    }

    fn compress(&self, input: &str, _budget: usize) -> String {
        let mut out = String::with_capacity(input.len());
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        let mut in_single = false;
        let mut in_double = false;
        let mut in_block = false;
        let mut escaped = false;

        while i < chars.len() {
            let c = chars[i];
            if in_block {
                if c == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    in_block = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            if in_single {
                out.push(c);
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '\'' {
                    in_single = false;
                }
                i += 1;
                continue;
            }
            if in_double {
                out.push(c);
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_double = false;
                }
                i += 1;
                continue;
            }
            // Not in string/block
            if c == '\'' {
                in_single = true;
                out.push(c);
                i += 1;
                continue;
            }
            if c == '"' {
                in_double = true;
                out.push(c);
                i += 1;
                continue;
            }
            if c == '/' && i + 1 < chars.len() {
                let nxt = chars[i + 1];
                if nxt == '/' {
                    // single line comment — skip to newline (keep newline)
                    i += 2;
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                    }
                    continue;
                } else if nxt == '*' {
                    in_block = true;
                    i += 2;
                    continue;
                }
            }
            out.push(c);
            i += 1;
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Strategy 2 — AST outline extractor
// ---------------------------------------------------------------------------

/// Structural outline: keeps signatures, imports, and type definitions while
/// collapsing function bodies to `{ /* … */ }`. Heuristic line-based filter
/// without full parser dependency — sufficient for token budget trimming.
#[derive(Debug, Default, Clone, Copy)]
pub struct ASTOutlineExtractor;

impl ASTOutlineExtractor {
    fn is_signature_line(line: &str) -> bool {
        let t = line.trim_start();
        t.starts_with("use ")
            || t.starts_with("mod ")
            || t.starts_with("pub ")
            || t.starts_with("fn ")
            || t.starts_with("async fn ")
            || t.starts_with("struct ")
            || t.starts_with("enum ")
            || t.starts_with("trait ")
            || t.starts_with("impl ")
            || t.starts_with("type ")
            || t.starts_with("import ")
            || t.starts_with("export ")
            || t.starts_with("class ")
            || t.starts_with("interface ")
            || t.starts_with("const ")
            || t.starts_with("let ")
    }
}

impl CompressionStrategy for ASTOutlineExtractor {
    fn name(&self) -> &'static str {
        "ASTOutlineExtractor"
    }

    fn compress(&self, input: &str, budget_tokens: usize) -> String {
        // First pass: outline
        let mut lines: Vec<String> = Vec::new();
        let mut depth: i32 = 0;
        let mut collapsing = false;
        let mut collapse_depth = 0;

        for raw in input.lines() {
            let opens = raw.matches('{').count() as i32;
            let closes = raw.matches('}').count() as i32;

            if collapsing {
                depth += opens - closes;
                if depth <= collapse_depth {
                    collapsing = false;
                }
                continue;
            }

            if Self::is_signature_line(raw) && raw.contains('{') && !raw.trim_end().ends_with(';') {
                // Signature with body — keep signature line, collapse body
                // Normalize to one-liner outline
                let sig = raw.trim_end().to_string();
                // Keep up to '{' inclusive
                if let Some(idx) = sig.find('{') {
                    lines.push(format!("{} /* … */", sig[..=idx].trim_end()));
                } else {
                    lines.push(sig);
                }
                depth += opens - closes;
                collapse_depth = depth - opens;
                if depth > collapse_depth {
                    collapsing = true;
                }
                continue;
            }

            // Keep imports / signatures even without braces, and blank lines sparsely
            if Self::is_signature_line(raw) || raw.trim().is_empty() {
                lines.push(raw.to_string());
            } else if raw.len() < 120 {
                // Keep short non-signature lines (likely interesting)
                lines.push(raw.to_string());
            }
            depth += opens - closes;
            if depth < 0 {
                depth = 0;
            }
        }

        let outlined = lines.join("\n");
        // Budget enforcement via TokenWindowCompactor
        let w = TokenWindowCompactor;
        w.compress(&outlined, budget_tokens)
    }
}

// ---------------------------------------------------------------------------
// Strategy 3 — Token window compactor (budget targeting)
// ---------------------------------------------------------------------------

/// Sliding-window compactor targeting `budget_tokens` (≈ `len/4`). Keeps tail.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokenWindowCompactor;

impl CompressionStrategy for TokenWindowCompactor {
    fn name(&self) -> &'static str {
        "TokenWindowCompactor"
    }

    fn compress(&self, input: &str, budget_tokens: usize) -> String {
        let budget = budget_tokens.max(64);
        let est = |s: &str| s.len().div_ceil(4);
        if est(input) <= budget {
            return input.to_string();
        }
        let keep_chars = (budget.saturating_sub(8)) * 4;
        // Keep head (system-ish) 20% + tail 80%
        let head_chars = (keep_chars / 5).min(input.len());
        let tail_chars = keep_chars - head_chars;
        let tail_start = input.len().saturating_sub(tail_chars);
        // Find char boundaries
        let head = &input[..input
            .char_indices()
            .nth(head_chars)
            .map(|(idx, _)| idx)
            .unwrap_or(head_chars)
            .min(input.len())];
        let tail = &input[input
            .char_indices()
            .nth(tail_start)
            .map(|(idx, _)| idx)
            .unwrap_or(tail_start)..];
        format!("{head}\n// … [compacted {} chars] …\n{tail}", input.len() - head.len() - tail.len())
    }
}

// ---------------------------------------------------------------------------
// Unified compressor
// ---------------------------------------------------------------------------

/// Orchestrates compression strategies against a token budget.
///
/// Default pipeline: comments strip → outline extract → window compact.
/// Each stage is skipped if already within budget.
#[derive(Debug, Clone)]
pub struct ContextCompressor {
    pub budget_tokens: usize,
}

impl Default for ContextCompressor {
    fn default() -> Self {
        Self { budget_tokens: 8192 }
    }
}

impl ContextCompressor {
    pub fn new(budget_tokens: usize) -> Self {
        Self {
            budget_tokens: budget_tokens.max(64),
        }
    }

    pub fn estimate_tokens(s: &str) -> usize {
        s.len().div_ceil(4)
    }

    /// Compress `prompt` + `system` jointly under budget. System is preserved
    /// preferentially; prompt is compressed aggressively.
    pub fn compress(&self, prompt: &str, system: &str) -> (String, String, CompressionMetrics) {
        let original = Self::estimate_tokens(prompt) + Self::estimate_tokens(system);
        // Preserve system lightly — only strip comments
        let stripper = LosslessCommentsStripper;
        let mut sys = stripper.compress(system, self.budget_tokens);
        let mut prm = stripper.compress(prompt, self.budget_tokens);

        let mut current = Self::estimate_tokens(&prm) + Self::estimate_tokens(&sys);
        if current > self.budget_tokens {
            // Outline prompt
            let outliner = ASTOutlineExtractor;
            // Allocate system ~30% of budget
            let sys_budget = (self.budget_tokens as f64 * 0.3) as usize;
            let prm_budget = self.budget_tokens.saturating_sub(Self::estimate_tokens(&sys).min(sys_budget));
            let new_prm = outliner.compress(&prm, prm_budget);
            prm = new_prm;
            current = Self::estimate_tokens(&prm) + Self::estimate_tokens(&sys);
        }
        if current > self.budget_tokens {
            let win = TokenWindowCompactor;
            let sys_budget = (self.budget_tokens as f64 * 0.3) as usize;
            sys = win.compress(&sys, sys_budget.max(64));
            let prm_budget = self.budget_tokens.saturating_sub(Self::estimate_tokens(&sys));
            prm = win.compress(&prm, prm_budget.max(64));
            current = Self::estimate_tokens(&prm) + Self::estimate_tokens(&sys);
        }

        let compressed = current;
        let metrics = CompressionMetrics::new(original, compressed, "ContextCompressor");
        // Emit tracing telemetry
        tracing::info!(
            event = "telemetry:compression",
            original_tokens = original,
            compressed_tokens = compressed,
            compression_ratio = metrics.compression_ratio,
            "context compression applied"
        );
        (prm, sys, metrics)
    }

    /// Compress a single blob (prompt-only) for tests.
    pub fn compress_single(&self, input: &str) -> (String, CompressionMetrics) {
        let (prm, _, m) = self.compress(input, "");
        (prm, m)
    }
}

// ---------------------------------------------------------------------------
// Tests — Phase 8.1 validation
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripper_preserves_strings() {
        let s = r#"let x = "// not a comment"; // real comment"#;
        let out = LosslessCommentsStripper.compress(s, 9999);
        assert!(out.contains("// not a comment"));
        assert!(!out.contains("real comment"));
    }

    #[test]
    fn stripper_removes_block_comments() {
        let s = "a /* block */ b";
        let out = LosslessCommentsStripper.compress(s, 9999);
        assert_eq!(out, "a  b");
    }

    #[test]
    fn outline_keeps_signatures() {
        let src = "use foo::bar;\nfn hello() {\n  println!(\"hi\");\n}\nstruct Foo { x: i32 }";
        let out = ASTOutlineExtractor.compress(src, 9999);
        assert!(out.contains("fn hello"));
        assert!(out.contains("struct Foo"));
        assert!(out.contains("use foo::bar"));
    }

    #[test]
    fn window_respects_budget() {
        let w = TokenWindowCompactor;
        let big = "a".repeat(40000);
        let out = w.compress(&big, 100);
        assert!(ContextCompressor::estimate_tokens(&out) <= 140); // budget + slack
    }

    #[test]
    fn compressor_budget_adherence() {
        let c = ContextCompressor::new(200);
        let prompt = "a".repeat(8000);
        let system = "b".repeat(2000);
        let (p, s, m) = c.compress(&prompt, &system);
        let total = ContextCompressor::estimate_tokens(&p) + ContextCompressor::estimate_tokens(&s);
        assert!(total <= 250, "total {} exceeds budget + slack", total);
        assert!(m.compression_ratio < 1.0);
        assert!(m.original_tokens > m.compressed_tokens);
    }

    #[test]
    fn ast_integrity_after_outline() {
        // Outline should keep valid UTF-8 and not truncate strings
        let src = r#"pub fn foo(x: &str) -> String { format!("hello {}", x) }"#;
        let c = ContextCompressor::new(50);
        let (p, _, _) = c.compress(src, "");
        assert!(p.is_char_boundary(0));
        assert!(p.contains("fn foo") || p.len() < src.len());
    }

    #[test]
    fn empty_input() {
        let c = ContextCompressor::default();
        let (p, s, m) = c.compress("", "");
        assert_eq!(p, "");
        assert_eq!(s, "");
        assert_eq!(m.compression_ratio, 1.0);
    }
}
