//! Live provider commissioning probe (Master Execution Baseline §11).
//!
//! Ignored by default so the normal battery never touches the network. Run
//! explicitly with:
//!
//! ```sh
//! cargo test -p zylcode-core --test live_commissioning -- --ignored --nocapture
//! ```
//!
//! Classification this test establishes (never collapsed into "provider
//! works"): `configured` → `authenticated` → `request sent` →
//! `HTTP response received` → `completion received`. It does NOT establish
//! streaming (the router chunks completed responses post-hoc), provider-native
//! tool calling, or multimodality.
//!
//! Invariants asserted here (the test FAILS only when the result would be a
//! lie):
//! * a persisted cache/vector-cache/synthetic payload must never be reported
//!   as a provider response;
//! * an answer faster than any real round-trip is rejected;
//! * an external condition (e.g. account balance) yields an honest BLOCKED
//!   classification, not a failure.
//!
//! Secrets: API keys are read by the router from the environment through the
//! application's intended path and are never printed, logged, or asserted on.

use std::time::Instant;

use uuid::Uuid;
use zylcode_core::router::{ModelProvider, RouterConfig, TokenRouter};

fn looks_synthetic(text: &str) -> bool {
    text.contains("Synthetic offline response") || text.contains("synthetic offl")
}

/// What one provider probe actually established.
enum ProbeOutcome {
    /// A real completion came back from the provider.
    Live,
    /// The request reached the provider and got a real HTTP answer
    /// (e.g. 402) — pipeline proven, provider blocked.
    HttpRefused,
    /// No real provider outcome (local failure, synthetic short-circuit,
    /// or a cache answer that would be a lie if reported as live).
    NoOutcome,
}

/// One live dispatch against one provider. Prints the classification.
async fn probe(provider: ModelProvider, model: &str, label: &str) -> ProbeOutcome {
    let config = RouterConfig {
        primary_provider: provider,
        primary_model: model.to_string(),
        // No fallback: a fallback answer must never be mistaken for the
        // primary provider's, and Ollama is not running here.
        max_retries: 0,
        ..Default::default()
    };
    // `without_vector_cache`: TokenRouter::new loads an on-disk vector cache
    // whose `mock_embed` similarity (char/bigram hashing, 0.88 threshold)
    // satisfied a first probe in 0.01s with a PERSISTED synthetic response
    // before any network attempt (observed 2026-09-28). Commissioning must
    // bypass it.
    let router = match TokenRouter::without_vector_cache(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{label}: configured, but router construction failed: {e:#}");
            return ProbeOutcome::NoOutcome;
        }
    };

    let nonce = Uuid::new_v4().to_string();
    let prompt =
        format!("Commissioning probe {nonce}. Reply with exactly the single word: COMMISSIONED");
    eprintln!("{label}: configured=YES (key present in environment, value not shown); sending…");
    let started = Instant::now();
    match router
        .dispatch_prompt(
            &prompt,
            "You are a commissioning probe. Reply with exactly one word and nothing else.",
        )
        .await
    {
        Ok(text) => {
            let latency = started.elapsed();
            let trimmed = text.trim();
            if looks_synthetic(trimmed) {
                eprintln!(
                    "{label}: NOT LIVE — router returned a synthetic payload in {latency:?}; \
                     commissioning NOT established for this provider"
                );
                return ProbeOutcome::NoOutcome;
            }
            if latency.as_millis() < 50 {
                eprintln!(
                    "{label}: NOT LIVE — returned in {latency:?}, too fast for a network \
                     round-trip; a cache answered, not the provider"
                );
                return ProbeOutcome::NoOutcome;
            }
            eprintln!(
                "{label}: authenticated=YES request_sent=YES response_received=YES \
                 completion_received=YES in {:.2}s; first 80 chars: {:.80}",
                latency.as_secs_f64(),
                trimmed.replace('\n', " ")
            );
            ProbeOutcome::Live
        }
        Err(e) => {
            let elapsed = started.elapsed();
            let msg = format!("{e:#}");
            if msg.contains("[status=") {
                // A real HTTP status from the real endpoint: the request left
                // this machine and the provider answered it.
                eprintln!(
                    "{label}: request reached provider=YES — HTTP response in \
                     {elapsed:?}, but NO completion: {msg}"
                );
                ProbeOutcome::HttpRefused
            } else {
                eprintln!(
                    "{label}: request attempted ({elapsed:?}) but no provider HTTP outcome: \
                     {msg}"
                );
                ProbeOutcome::NoOutcome
            }
        }
    }
}

#[tokio::test]
#[ignore = "makes real provider calls; run explicitly with --ignored"]
async fn live_provider_commissioning_probe() {
    // Candidate order: cheapest real completion first. Keys are checked for
    // presence only — values are never touched by this test.
    let mut any_live = false;
    let mut any_http_outcome = false;

    let candidates: Vec<(ModelProvider, &str, &str)> = vec![
        // The Respan hosted gateway probes first when its key is present: it is
        // the only candidate whose request is traced server-side, so a live
        // completion here also proves the path the integration exists for.
        (
            ModelProvider::Respan,
            zylcode_core::router::RESPAN_DEFAULT_MODEL,
            "[Respan gateway]",
        ),
        (ModelProvider::DeepSeek, "deepseek-chat", "[DeepSeek]"),
        (
            ModelProvider::OpenRouter,
            "deepseek/deepseek-chat:free",
            "[OpenRouter free tier]",
        ),
        (
            ModelProvider::Anthropic,
            "claude-3-5-haiku-latest",
            "[Anthropic direct]",
        ),
    ];

    for (provider, model, label) in candidates {
        let key_env = match provider {
            ModelProvider::DeepSeek => "DEEPSEEK_API_KEY",
            ModelProvider::OpenRouter => "OPENROUTER_API_KEY",
            ModelProvider::Anthropic => "ANTHROPIC_API_KEY",
            ModelProvider::Respan => "RESPAN_API_KEY",
            _ => "",
        };
        let present = std::env::var(key_env)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        if !present {
            eprintln!("{label}: configured=NO ({key_env} not set) → BLOCKED");
            continue;
        }
        match probe(provider, model, label).await {
            ProbeOutcome::Live => {
                any_live = true;
                any_http_outcome = true;
                break;
            }
            ProbeOutcome::HttpRefused => {
                any_http_outcome = true;
            }
            ProbeOutcome::NoOutcome => {}
        }
    }

    if any_live {
        eprintln!("OVERALL: live completion received through the intended path → LIVE COMMISSIONING VERIFIED (single completion only)");
    } else if any_http_outcome {
        eprintln!(
            "OVERALL: real provider HTTP outcomes captured, but NO completion received → \
             live commissioning BLOCKED (see per-provider reasons above)"
        );
    } else {
        eprintln!(
            "OVERALL: no provider HTTP outcome at all → live commissioning UNVERIFIED/ BLOCKED"
        );
    }
    eprintln!("STREAMING: not established — dispatch_stream re-chunks a completed response post-hoc (no SSE).");
    eprintln!("PROVIDER-NATIVE TOOL CALLING: not established — request bodies carry no `tools` parameter.");
    eprintln!("MULTIMODAL: not established — not probed.");
    // Deliberately no pass/fail assertion on the classification: external
    // conditions (missing keys, account balance, offline network) are BLOCKED
    // or UNVERIFIED states, not defects in this probe. The invariant that
    // matters — never reporting a synthetic/cache answer as a provider
    // response — is enforced inside probe() before anything is printed.
}
