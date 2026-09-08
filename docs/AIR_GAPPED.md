# Air-Gapped Mode

ZylCode supports fully offline operation for environments where network access is restricted or unavailable. This mode routes all requests to a local `SyntheticOffline` provider that generates deterministic responses without external API calls.

## Overview

When running in air-gapped mode, ZylCode:

- **Skips all network requests** — no telemetry, no provider fallback, no model calls
- **Uses SyntheticOffline provider** — generates deterministic, local-only responses
- **Zero cost** — all tokens are synthetic, no billing incurred
- **Preserves verification rungs** — lint, type-check, and property tests still run locally
- **Maintains audit trail** — all operations are logged locally

## Usage

### CLI Flag

Pass `--offline` to the `verify` subcommand:

```bash
zylcode-core-cli verify --workspace /path/to/project --offline
```

### Environment Variable

Set `ZYLCODE_OFFLINE=1` for persistent offline mode:

```bash
export ZYLCODE_OFFLINE=1
zylcode-core-cli verify --workspace /path/to/project
```

### Programmatic API

When using the Rust library directly, configure the router with the offline provider:

```rust
use zylcode_core::router::RouterConfig;

let config = RouterConfig {
    primary_provider: "synthetic_offline".to_string(),
    fallback_provider: "synthetic_offline".to_string(),
    ..Default::default()
};
```

## Provider Behavior

### SyntheticOffline Provider

The offline provider:

- Returns deterministic, placeholder responses
- Does not call any external APIs or services
- Generates zero tokens (no billing)
- Provides mock verification results

### Pricing

| Provider | Input Cost | Output Cost |
|---|---|---|
| Anthropic | $3.00 / 1M tokens | $15.00 / 1M tokens |
| OpenRouter | $0.15 / 1M tokens | $0.60 / 1M tokens |
| Ollama | Free | Free |
| SyntheticOffline | Free | Free |

**Note:** Ollama remains free but requires a running local Ollama instance. SyntheticOffline requires no external services.

## Verification in Offline Mode

All verification rungs operate locally:

- **Rung 1 (Lint):** Runs `cargo clippy`, `tsc --noEmit`, `mypy` — requires toolchain installed locally
- **Rung 2 (Property Tests):** Runs `proptest`, `fast-check` — generates and executes tests locally
- **Rung 3 (Formal Spec):** Runs Alloy, TLA+ — checks specifications locally
- **Rung 4 (Full Verification):** Runs Dafny, Z3 — machine-checked proofs locally

### Limitations

- No network-dependent linters or formatters
- No external model fallback if local tools fail
- No real-time security advisory updates
- Limited to locally installed verification tools

## Configuration

### Offline Mode Settings

```rust
use zylcode_core::router::RouterConfig;

let config = RouterConfig {
    primary_provider: "synthetic_offline".into(),
    fallback_provider: "synthetic_offline".into(),
    timeout_ms: 5000,
    max_retries: 0,  // No retries in offline mode
    ..Default::default()
};
```

### Fallback Behavior

When `--offline` is set:

1. All provider routing skips network calls
2. Fallback chain is disabled (single provider only)
3. Telemetry submission is deferred (cached locally)
4. Audit logs are written to local disk

## Use Cases

### Air-Gapped Networks

For classified or high-security environments:

```bash
# Run in complete isolation
ZYLCODE_OFFLINE=1 zylcode-core-cli verify \
  --workspace /secure/project \
  --offline \
  --verbose
```

### CI/CD Pipelines

For build systems without internet access:

```yaml
# .gitlab-ci.yml
verify:
  script:
    - ZYLCODE_OFFLINE=1 zylcode-core-cli verify --offline
```

### Disconnected Development

For laptops or remote locations without connectivity:

```bash
# Cache verification results locally
zylcode-core-cli verify --offline --output results.json

# Review cached results later
zylcode-core-cli verify --offline --verbose 2>&1 | tee verification.log
```

## Troubleshooting

### Common Issues

**"No provider available" error:**
- Ensure `--offline` flag is set
- Check that `synthetic_offline` is configured as primary provider

**Verification tools not found:**
- Install required toolchains (rustup, npm, python)
- Verify tools are in PATH

**Audit logs not writing:**
- Check write permissions to `~/.zylcode/audit/`
- Ensure disk space is available

### Debug Mode

Enable verbose logging for offline diagnostics:

```bash
ZYL_LOG=debug zylcode-core-cli verify --offline --verbose
```

## Security Considerations

- **No data exfiltration:** All processing remains local
- **Audit trail preserved:** All operations logged locally
- **Deterministic responses:** Mock verification results are predictable
- **No external dependencies:** No network calls during execution

## Future Enhancements

- Local model inference via Ollama integration
- Offline security advisory database
- Encrypted local audit log storage
- Air-gapped update mechanism for verification tools
