//! The runtime path for provider-native tool calling.
//!
//! Before this module, a provider's `tool_call` reached the neutral layer and
//! stopped there: nothing forwarded it to ZylCode's own tool pipeline, so
//! "native tool calling" meant "we accept the field". This closes the loop:
//!
//! ```text
//! request with declared tools, depsite model, provider tool call
//!   → canonical id resolution (wire name → ZylCode id, fail closed)
//!   → argument validation (a JSON object, or the call is refused)
//!   → real_tools::dispatch (permission gate + ToolRegistry executor + evidence)
//!   → native tool result
//!   → same conversation, next round
//! ```
//!
//! There is deliberately no second executor here. `dispatch` is the single gate
//! every other product surface uses, so a model-requested call cannot reach an
//! executor that a human-requested call could not.
//!
//! Fail-closed rules, all of them tested:
//! - a wire name that is not among the declared tools ends the run; no tool is
//!   invented and nothing is executed;
//! - a call whose arguments are not a JSON object is refused before the gate;
//! - a refusal by the gate is recorded as evidence and reported to the model as
//!   an error result rather than silently dropped;
//! - the round limit ends the run instead of looping forever.

use anyhow::Result;

use crate::providers::streaming::{
    wire_name, CancelToken, ChatMessage, ChatRequest, FinishReason, ProviderError, StreamOutcome,
    ToolCall, ToolChoice, ToolSpec,
};
use zylcode_mcp::evidence::ToolRuntime;
use zylcode_mcp::real_tools::{dispatch, ToolContext, ToolEvidence};

/// The maximum number of model rounds one run may take.
///
/// A native tool conversation is a loop, and a loop needs a bound: a model that
/// keeps asking for a tool would otherwise spend the owner's credit forever.
pub const MAX_TOOL_ROUNDS: usize = 8;

/// Largest tool output forwarded back to a model, in characters.
///
/// Over-long output is truncated with an explicit marker: silently cutting it
/// would let the model answer as if it had read the whole thing.
pub const MAX_TOOL_RESULT_CHARS: usize = 16_000;

/// One provider round. Implemented by the router for real calls and by tests
/// for scripted ones, so the loop's rules can be tested without a network.
#[async_trait::async_trait]
pub trait ChatTurn: Send + Sync {
    async fn turn(
        &self,
        request: &ChatRequest,
        cancel: &CancelToken,
    ) -> std::result::Result<StreamOutcome, ProviderError>;
}

/// Everything one native run executed, in order.
#[derive(Debug, Clone)]
pub struct ExecutedCall {
    /// The name the provider used.
    pub wire_name: String,
    /// The ZylCode id it resolved to. Never equal to `wire_name` for a dotted id.
    pub tool_id: String,
    pub call_id: Option<String>,
    /// The gate's decision, as recorded in evidence.
    pub decision: String,
    /// Whether the executor actually ran (the gate allowed it and it returned Ok).
    pub executed: bool,
    /// What was sent back to the model as the tool result.
    pub result_content: String,
    /// The evidence record for this invocation.
    pub evidence: ToolEvidence,
}

/// The outcome of a completed native run.
#[derive(Debug, Clone)]
pub struct NativeRun {
    /// The assistant's final text.
    pub text: String,
    pub finish: FinishReason,
    pub rounds: usize,
    pub calls: Vec<ExecutedCall>,
}

impl NativeRun {
    /// Invocation ids of every attempt, including refusals.
    pub fn invocation_ids(&self) -> Vec<&str> {
        self.calls
            .iter()
            .map(|c| c.evidence.invocation_id.as_str())
            .collect()
    }
}

/// Why a native run stopped early.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeToolError {
    /// The provider failed, or the transport did.
    Provider(String),
    /// The model asked for a tool that was never declared.
    UnknownTool { wire_name: String },
    /// The model's call could not be turned into an invocation.
    MalformedCall { wire_name: String, reason: String },
    /// Two declared tools collapse to one wire name, so a returned call would
    /// be ambiguous. Refused before any request is sent.
    AmbiguousDeclaration { wire_name: String },
    /// The run exceeded [`MAX_TOOL_ROUNDS`].
    RoundLimit(usize),
    /// The caller cancelled.
    Cancelled,
}

impl std::fmt::Display for NativeToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider(message) => write!(f, "provider failed: {message}"),
            Self::UnknownTool { wire_name } => write!(
                f,
                "model called `{wire_name}`, which was not among the declared tools; \
                 refusing to invent a tool"
            ),
            Self::MalformedCall { wire_name, reason } => {
                write!(f, "model's call to `{wire_name}` was malformed: {reason}")
            }
            Self::AmbiguousDeclaration { wire_name } => write!(
                f,
                "two tools declare the same wire name `{wire_name}`; \
                 a returned call could not be resolved unambiguously"
            ),
            Self::RoundLimit(rounds) => {
                write!(f, "run exceeded {rounds} model rounds without finishing")
            }
            Self::Cancelled => write!(f, "run cancelled"),
        }
    }
}

impl std::error::Error for NativeToolError {}

/// The router as a provider round, so a real run and a scripted test reach
/// the same loop through the same trait.
#[async_trait::async_trait]
impl ChatTurn for crate::router::TokenRouter {
    async fn turn(
        &self,
        request: &ChatRequest,
        cancel: &CancelToken,
    ) -> std::result::Result<StreamOutcome, ProviderError> {
        self.stream_turn(request, cancel).await
    }
}

/// The tools ZylCode can actually execute, as provider declarations.
///
/// Sourced from the same catalogue the product surface reports, filtered to
/// entries with an executor, so a declaration can never promise a tool that
/// `dispatch` would refuse to find.
pub fn tool_declarations() -> std::result::Result<Vec<ToolSpec>, NativeToolError> {
    let catalogue = zylcode_mcp::tool_catalogue::Catalogue::canonical();
    let mut specs: Vec<ToolSpec> = Vec::new();
    for entry in catalogue.entries() {
        if !entry.status.is_executable()
            || zylcode_mcp::real_tools::get_real_tool(&entry.id).is_none()
        {
            continue;
        }
        specs.push(ToolSpec {
            name: entry.id.clone(),
            description: entry.description.clone(),
            parameters: entry.input_schema.clone(),
        });
    }

    // A deterministic mapping is only useful if it is a function; check that
    // before a request goes out, not when a call comes back.
    let mut seen: Vec<String> = Vec::new();
    for spec in &specs {
        let wire = spec.wire_name();
        if seen.contains(&wire) {
            return Err(NativeToolError::AmbiguousDeclaration { wire_name: wire });
        }
        seen.push(wire);
    }
    Ok(specs)
}

/// Resolve the name a provider returned to the declaration that produced it.
pub fn resolve<'a>(specs: &'a [ToolSpec], returned: &str) -> Option<&'a ToolSpec> {
    specs
        .iter()
        .find(|spec| spec.wire_name() == returned || spec.name == returned)
}

/// Everything a run needs to reach the provider and the tool pipeline.
pub struct NativeToolRunner<'a> {
    turn: &'a dyn ChatTurn,
    runtime: &'a ToolRuntime,
    context: ToolContext,
    system: String,
    tools: Vec<ToolSpec>,
    max_rounds: usize,
}

impl<'a> NativeToolRunner<'a> {
    pub fn new(turn: &'a dyn ChatTurn, runtime: &'a ToolRuntime, context: ToolContext) -> Self {
        Self {
            turn,
            runtime,
            context,
            system: String::new(),
            tools: Vec::new(),
            max_rounds: MAX_TOOL_ROUNDS,
        }
    }

    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = system.into();
        self
    }

    /// Declare an explicit tool set. Defaults to [`tool_declarations`].
    pub fn with_tools(mut self, tools: Vec<ToolSpec>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_max_rounds(mut self, rounds: usize) -> Self {
        self.max_rounds = rounds.max(1);
        self
    }

    /// Run one task to completion, executing whatever the model asks for
    /// through the gated pipeline.
    pub async fn run(
        &self,
        prompt: &str,
        model: &str,
        cancel: &CancelToken,
    ) -> std::result::Result<NativeRun, NativeToolError> {
        let tools = if self.tools.is_empty() {
            tool_declarations()?
        } else {
            self.tools.clone()
        };

        let mut messages: Vec<ChatMessage> = vec![ChatMessage::User {
            content: prompt.to_string(),
        }];
        let mut calls: Vec<ExecutedCall> = Vec::new();

        for round in 1..=self.max_rounds {
            if cancel.is_cancelled() {
                return Err(NativeToolError::Cancelled);
            }

            let request = ChatRequest::new(model, self.system.clone(), "")
                .with_tools(tools.clone())
                .with_messages(messages.clone())
                .with_tool_choice(ToolChoice::Auto);

            let outcome = self
                .turn
                .turn(&request, cancel)
                .await
                .map_err(|err| NativeToolError::Provider(err.message))?;

            if cancel.is_cancelled() {
                return Err(NativeToolError::Cancelled);
            }

            if outcome.tool_calls.is_empty() {
                return Ok(NativeRun {
                    text: outcome.text,
                    finish: outcome.finish,
                    rounds: round,
                    calls,
                });
            }

            // The assistant turn has to be carried back verbatim, including the
            // text it produced alongside the calls.
            messages.push(ChatMessage::Assistant {
                content: outcome.text.clone(),
                tool_calls: outcome.tool_calls.clone(),
            });

            for call in &outcome.tool_calls {
                let (executed, message) = self.execute_call(&tools, call, &mut calls).await?;
                messages.push(ChatMessage::ToolResult {
                    call_id: message.call_id,
                    name: message.name,
                    content: message.content,
                    is_error: !executed,
                });
            }
        }

        Err(NativeToolError::RoundLimit(self.max_rounds))
    }

    /// Gate and execute one call. Returns whether it ran, plus the message to
    /// send back to the model.
    async fn execute_call(
        &self,
        tools: &[ToolSpec],
        call: &ToolCall,
        calls: &mut Vec<ExecutedCall>,
    ) -> std::result::Result<(bool, ToolResultMessage), NativeToolError> {
        let Some(spec) = resolve(tools, &call.name) else {
            return Err(NativeToolError::UnknownTool {
                wire_name: call.name.clone(),
            });
        };

        let arguments: serde_json::Value =
            serde_json::from_str(&call.arguments).map_err(|err| {
                NativeToolError::MalformedCall {
                    wire_name: call.name.clone(),
                    reason: format!("arguments are not JSON: {err}"),
                }
            })?;
        if !arguments.is_object() {
            return Err(NativeToolError::MalformedCall {
                wire_name: call.name.clone(),
                reason: "arguments must be a JSON object".to_string(),
            });
        }

        // A result message must be addressable back to the call that asked for
        // it. Anthropic and the OpenAI family both reject a result without an id.
        let call_id = call
            .call_id
            .clone()
            .or_else(|| call.block_id.clone())
            .ok_or_else(|| NativeToolError::MalformedCall {
                wire_name: call.name.clone(),
                reason: "the provider did not supply a call id".to_string(),
            })?;

        let outcome = dispatch(&spec.name, arguments, &self.context, self.runtime).await;
        let decision = outcome.decision.evidence_record();
        let executed = outcome.executed();
        let content = render_result(&outcome.result);

        calls.push(ExecutedCall {
            wire_name: call.name.clone(),
            tool_id: spec.name.clone(),
            call_id: Some(call_id.clone()),
            decision,
            executed,
            result_content: content.clone(),
            evidence: outcome.evidence,
        });

        Ok((
            executed,
            ToolResultMessage {
                call_id,
                name: spec.name.clone(),
                content,
            },
        ))
    }
}

struct ToolResultMessage {
    call_id: String,
    name: String,
    content: String,
}

/// What the model is told a tool produced.
///
/// A failing tool reports its error as the result instead of aborting the run:
/// the model asked, the gate allowed, the executor refused — that is information
/// it needs in order to continue sensibly.
fn render_result(result: &Result<zylcode_mcp::real_tools::ToolResult>) -> String {
    let content = match result {
        Ok(result) => serde_json::to_string(&result.output).unwrap_or_else(|_| "{}".to_string()),
        Err(err) => format!("tool error: {err}"),
    };
    truncate(&content, MAX_TOOL_RESULT_CHARS)
}

fn truncate(content: &str, limit: usize) -> String {
    if content.chars().count() <= limit {
        return content.to_string();
    }
    let kept: String = content.chars().take(limit).collect();
    format!("{kept}\n… output truncated at {limit} characters by ZylCode; the tool produced more.")
}

/// The runtime a real run uses: the restrictive policy and the JSONL ledger.
///
/// Not a permissive test runtime — a model-requested call is subject to exactly
/// the policy a human-requested one is.
pub fn default_runtime() -> ToolRuntime {
    ToolRuntime::default()
}

/// The wire name a canonical id is declared under. Exposed so the CLI can show
/// the mapping it used without duplicating the rule.
pub fn declared_name(id: &str) -> String {
    wire_name(id)
}

#[cfg(test)]
#[path = "native_tools_tests.rs"]
mod native_tools_tests;
