//! The provider boundary: what a request to a model *is*, and the clients
//! that carry one.
//!
//! `AgentLoop` owns the decision logic; this module owns the seam in front of
//! the provider. Splitting it out keeps the dispatch contract — two strings,
//! assembled locally, handed over untouched — in one place to read, and keeps
//! [`repository_context_block`]'s deterministic citation budget testable
//! without constructing an agent.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::router::TokenRouter;

/// Trait for model clients
#[async_trait]
pub trait ModelClient: Send + Sync {
    async fn call(&self, prompt: &str, system: &str) -> Result<String>;
}

/// Real model client using TokenRouter
pub struct RealModelClient {
    router: Arc<TokenRouter>,
}

impl RealModelClient {
    pub fn new(router: Arc<TokenRouter>) -> Self {
        Self { router }
    }
}

#[async_trait]
impl ModelClient for RealModelClient {
    async fn call(&self, prompt: &str, system: &str) -> Result<String> {
        self.router.dispatch_prompt(prompt, system).await
    }
}

/// Test model client for deterministic testing
pub struct TestModelClient {
    responses: Vec<String>,
    index: std::sync::atomic::AtomicUsize,
}

impl TestModelClient {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            index: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl ModelClient for TestModelClient {
    async fn call(&self, _prompt: &str, _system: &str) -> Result<String> {
        let idx = self.index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if idx < self.responses.len() {
            Ok(self.responses[idx].clone())
        } else {
            // Return a default response
            Ok(r#"{"action": "Complete", "payload": {"summary": "Task completed", "evidence": [], "remaining_limitations": []}}"#.to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Provider-bound request (G3)
//
// The model never sees `session.messages`: every request is assembled
// locally from the task, the plan and this loop's gathered intelligence, and
// then handed to `ModelClient::call` as two strings. Repository-intelligence
// citations therefore had to be carried on the *session* — not merely stored
// as a message — or they stop at the boundary exactly where the previous
// commissioning found them stopping.
// ---------------------------------------------------------------------------

/// System prompt sent with every provider-bound request.
pub(super) const BASE_SYSTEM_PROMPT: &str = "You are an expert software engineering agent. You must respond with valid JSON matching the AgentDecision protocol.";

/// Upper bound on citation lines carried by one request.
///
/// `ContextBuilder` gathers with the same limit, so this is the belt to the
/// gatherer's braces: even a future caller that gathers more cannot push an
/// unbounded block through this boundary.
pub const REPO_CONTEXT_MAX_CITATIONS: usize = 12;

/// Byte budget for the rendered citation block, applied to **whole lines**.
///
/// A citation is never cut in half — `file:line` integrity is the whole point
/// of the block — so overflowing lines are dropped and the drop is stated
/// rather than silent.
pub const REPO_CONTEXT_MAX_CHARS: usize = 4_096;

/// The two strings handed to the provider. This *is* the dispatch boundary:
/// [`super::AgentLoop::call_model`] passes them to [`ModelClient::call`] unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRequest {
    pub prompt: String,
    pub system: String,
}

/// Render gathered citations as the block prepended to every request.
///
/// Returns `None` when there is nothing to say — never an empty block, which
/// would read to a model as "no relationships exist". Truncation is
/// deterministic (first N whole lines within the byte budget) and always
/// announced in the output, so a reviewer can see exactly what was dropped.
pub fn repository_context_block(citations: &[String]) -> Option<String> {
    if citations.is_empty() {
        return None;
    }

    let mut kept: Vec<&str> = Vec::new();
    let mut used = 0usize;
    let mut dropped = 0usize;

    for (index, line) in citations.iter().enumerate() {
        if index >= REPO_CONTEXT_MAX_CITATIONS {
            dropped += citations.len() - index;
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // +1 for the newline that will join it.
        if used + line.len() + 1 > REPO_CONTEXT_MAX_CHARS {
            if kept.is_empty() {
                // Never drop *everything*: one whole citation is always
                // cheaper than an unbounded claim.
                kept.push(line);
                dropped += citations.len() - index - 1;
                break;
            }
            dropped += citations.len() - index;
            break;
        }
        used += line.len() + 1;
        kept.push(line);
    }

    if kept.is_empty() {
        return None;
    }

    // The header has to describe what is actually below it: calling a single
    // "no evidence exists" marker "Deterministic evidence resolved from the
    // parsed repository index" would be the same class of overstatement the
    // wave forbids everywhere else.
    let carries_evidence = kept.iter().any(|line| line.starts_with("DETERMINISTIC"));
    let mut block = String::from(if carries_evidence {
        "REPOSITORY INTELLIGENCE CONTEXT\n\n\
         Deterministic evidence resolved from the parsed repository index:\n"
    } else {
        "REPOSITORY INTELLIGENCE CONTEXT\n\n\
         No graph evidence is available for this task:\n"
    });
    for line in &kept {
        block.push_str(line);
        block.push('\n');
    }
    if dropped > 0 {
        block.push_str(&format!(
            "… {dropped} further citation line{} omitted by the repository-context \
             budget ({} lines / {} bytes) — their absence is a budget limit, not a \
             finding.\n",
            if dropped == 1 { "" } else { "s" },
            REPO_CONTEXT_MAX_CITATIONS,
            REPO_CONTEXT_MAX_CHARS,
        ));
    }
    block.push_str(
        "\nInstructions:\n\
         - Treat the lines above as repository evidence for this task.\n\
         - Cite the `file:line` locations above when you make a repository-specific claim.\n\
         - Do not claim a file, symbol or relationship that is not covered above.\n\
         - If the lines above say no graph evidence exists, do not invent one.\n\
         - Anything not covered above is your own interpretation — label it as such, and \
         never present an inference as a graph fact.",
    );
    Some(block)
}

/// Prepend the repository block to `prompt`, when there is one.
pub(super) fn with_repository_context(prompt: &str, citations: &[String]) -> String {
    match repository_context_block(citations) {
        Some(block) => format!("{block}\n\n{prompt}"),
        None => prompt.to_string(),
    }
}
