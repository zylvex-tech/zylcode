# Compression — Phase 8.1 Context Compaction Engine

## Goals

- Target token budget (`context_window_tokens`, default 8192) before egress.
- Reduce egress tokens without breaking artifact parseability.
- Preserve system prompt (≈30% budget) and prompt tail (most recent intent).

## Module

`crates/zylcode-core/src/compression.rs` — pure Rust, no parser deps.

### ContextCompressor

```rust
pub struct ContextCompressor { pub budget_tokens: usize } // default 8192, min 64
impl ContextCompressor {
  pub fn new(budget_tokens: usize) -> Self;
  pub fn estimate_tokens(s: &str) -> usize { s.len().div_ceil(4) }
  pub fn compress(&self, prompt: &str, system: &str) -> (String, String, CompressionMetrics);
  pub fn compress_single(&self, input: &str) -> (String, CompressionMetrics);
}
pub struct CompressionMetrics { 
  pub original_tokens: usize, 
  pub compressed_tokens: usize, 
  pub compression_ratio: f64, 
  pub strategy: String 
}
```

Pipeline in `compress(prompt, system)`:

1. `LosslessCommentsStripper` on both prompt and system.
2. If `tokens(prompt)+tokens(system) > budget` → `ASTOutlineExtractor` on prompt (budget = total − 30% for system).
3. If still over → `TokenWindowCompactor` on both (system 30% head, prompt tail remainder).
4. Emit `tracing::info!(event="telemetry:compression", original_tokens, compressed_tokens, compression_ratio)`.
5. Return `(compressed_prompt, compressed_system, metrics)`.

`estimate_tokens = len.div_ceil(4)` matches app-wide heuristic.

### Strategies

#### LosslessCommentsStripper

State machine preserving `'…'` and `"…"` with escape handling; strips `//…\n` (keeps newline) and `/* … */`. Property: `strip(strip(x)) == strip(x)`.

```
in:  let x = "// not a comment"; // real comment\n a /* block */ b
out: let x = "// not a comment"; \n a  b
```

#### ASTOutlineExtractor

Line-based heuristic:

- `is_signature_line` matches `use | mod | pub | fn | async fn | struct | enum | trait | impl | type | import | export | class | interface | const | let`.
- If line contains `{` and not `;`-terminated, emit `sig[..='{'] + " /* … */"` and enter collapsing mode until brace depth ≤ collapse_depth.
- Otherwise keep imports/signatures/blank/short (<120 chars) lines.
- Post-outline window compact against budget.

Effectively collapses bodies while preserving type graph.

#### TokenWindowCompactor

Budget-targeted sliding window:

- If `est(input) ≤ budget` → return input.
- Else `keep_chars = (budget−8)*4`; `head = keep_chars/5`, `tail = keep_chars−head`; output = `head + "\n// … [compacted N chars] …\n" + tail` with char-boundary slicing.

Tuning: keep 20% head + 80% tail to preserve system-ish prefix and recent intent tail.

### Integration

`crates/zylcode-core/src/router.rs::TokenRouter::dispatch_prompt`:

```rust
let budget = self.config.context_window_tokens as usize;
let (prompt_owned, system_owned) = {
  let est = |s: &str| s.len().div_ceil(4);
  if est(prompt) + est(system) > budget {
    let compressor = crate::compression::ContextCompressor::new(budget);
    let (p, s, _m) = compressor.compress(prompt, system);
    (p, s)
  } else {
    trim_to_window(prompt, system, self.config.context_window_tokens, self.config.trim_strategy)
  }
};
```

- Cache key uses compressed prompt/system + primary_model.
- `dispatch_stream` delegates to `dispatch_prompt`, so compression benefits streaming too.

Telemetry hook:

- `telemetry:compression` tracing fields `original_tokens`, `compressed_tokens`, `compression_ratio`.
- UI can `listen("telemetry:compression", …)`; or tail logs via `tracing_subscriber`.

### Tests (7)

Located in `compression.rs` `#[cfg(test)]`:

- `stripper_preserves_strings` — `//` inside `"…"` kept.
- `stripper_removes_block_comments` — `a /* block */ b → a  b`.
- `outline_keeps_signatures` — `use`, `fn`, `struct` retained.
- `window_respects_budget` — 40k `a`s + budget 100 → ≤140 tokens.
- `compressor_budget_adherence` — 8k+2k mixed → ≤250 total, ratio <1.
- `ast_integrity_after_outline` — UTF-8 boundary + contains `fn foo` or shrunk.
- `empty_input` — `""` → ratio 1.0.

Run:

```bash
cargo test --workspace -- compression
cargo test -p zylcode-core compression -- --nocapture
```

### Operational Knobs

- `RouterConfig::context_window_tokens` (default 8192) — bump for 32k models; shrink for edge LLM.
- `ContextCompressor::new(budget)` — instantiate per-request for dynamic budget.
- `ContextTrim::TruncateHead vs SlidingWindow` — fallback when under budget.

### Gotchas

- Stripping is lossless for code; natural prose `//` is treated as comment — acceptable for code-oriented prompts.
- Outline extractor is heuristic; full Rust/TS parser could be swapped by implementing `CompressionStrategy` for a `syn`/`swc`-backed struct.
- Window compactor inserts marker `// … [compacted N chars] …` that `parse_artifacts` ignores (not an `<artifact>`).
