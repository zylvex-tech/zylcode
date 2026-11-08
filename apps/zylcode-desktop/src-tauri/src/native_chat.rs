//! The desktop chat surface over ZylCode's one native tool loop.
//!
//! Before this module the desktop path ended at `router.dispatch_prompt`: the
//! model got text and nothing else, so a chat turn could never read a file or
//! run a tool, while `zylcode ask` could. This module is the seam that fixes
//! that, and it is deliberately thin:
//!
//! ```text
//! prompt → plan (IntentPlanner) → NativeToolRunner (core)
//!        → provider native tool call → canonical id → permission gate
//!        → ToolRegistry executor → evidence ledger → native tool result
//!        → continuation → final text
//! ```
//!
//! There is no desktop executor, no second gate and no second set of tool
//! declarations here. `NativeToolRunner` owns the loop; this module supplies
//! the context (workspace, actor, cancellation) and a sink for the app's own
//! evidence file, then reports each executed call to the UI.
//!
//! Evidence goes to the per-user settings directory, not the process working
//! directory: a packaged desktop app's CWD is wherever the launcher happened to
//! be, and a ledger that moves is not a ledger.

use std::path::{Path, PathBuf};

use zylcode_core::native_tools::{
    tool_context, CallObserver, ExecutedCall, NativeRun, NativeToolError, NativeToolRunner,
};
use zylcode_core::providers::streaming::CancelToken;
use zylcode_mcp::evidence::ToolRuntime;

/// Where the desktop writes its evidence ledger.
///
/// `ZYLCODE_CONFIG_DIR` wins so a portable install and the tests can redirect
/// it, exactly as the provider settings already do.
pub fn evidence_path() -> PathBuf {
    crate::provider_store::settings_dir().join("evidence.jsonl")
}

/// One desktop chat turn, executed through the shared native loop.
///
/// `on_call` receives every executed call after the gate and the executor have
/// run, so the UI row and the final report are the same record. It must not
/// block: it runs on the task driving the provider stream.
#[allow(clippy::too_many_arguments)]
pub async fn run_native_session(
    turn: &dyn zylcode_core::native_tools::ChatTurn,
    workspace: &Path,
    evidence_log: &Path,
    system: &str,
    prompt: &str,
    model: &str,
    actor: &str,
    session_id: String,
    cancel: &CancelToken,
    on_call: impl Fn(&ExecutedCall) + Send + Sync + 'static,
) -> Result<NativeRun, NativeToolError> {
    // The restrictive policy, and a sink in the application's own directory.
    // No permissive test runtime: a model-requested call is held to exactly the
    // standard a human-requested one is.
    let runtime = ToolRuntime::default().with_evidence_at(evidence_log);
    let context = tool_context(workspace, actor, Some(session_id));
    let observer: CallObserver = Box::new(on_call);

    NativeToolRunner::new(turn, &runtime, context)
        .with_system(system)
        .with_observer(observer)
        .run(prompt, model, cancel)
        .await
}

/// Whether the configured provider is allowed to be handed native tools.
///
/// A capability flag is the only place this decision may live. Enabling the
/// loop because it *exists* is how a surface ends up promising a provider
/// something it cannot do.
pub fn provider_supports_native_tools(provider: &zylcode_core::router::ModelProvider) -> bool {
    zylcode_core::providers::capabilities::provider_capabilities(provider).tools
}

/// A cancellation slot shared by the chat command and the cancel command.
///
/// One slot per app: starting a turn replaces the previous token, and a token
/// that is dropped does not stop anything, so the slot holds the live one.
#[derive(Default)]
pub struct CancelSlot {
    inner: std::sync::Mutex<Option<CancelToken>>,
}

impl CancelSlot {
    pub fn install(&self, token: CancelToken) {
        *self.inner.lock().expect("cancel slot poisoned") = Some(token);
    }

    /// Cancel the live turn, if there is one.
    pub fn cancel(&self) -> bool {
        let slot = self.inner.lock().expect("cancel slot poisoned");
        match slot.as_ref() {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }

    pub fn clear(&self) {
        *self.inner.lock().expect("cancel slot poisoned") = None;
    }
}

/// Turn the app handle's emitter into an observer without leaking the handle
/// type into this module's signature.
pub fn emit_call(app: &tauri::AppHandle, call: &ExecutedCall) {
    use tauri::Emitter;

    // The canonical id, not the provider's wire name: `fs_read` is a protocol
    // detail, `fs.read` is what the product calls the tool.
    let payload = serde_json::json!({
        "tool": call.tool_id,
        "wire_name": call.wire_name,
        "status": if call.executed { "complete" } else { "failed" },
        "decision": call.decision,
        "executed": call.executed,
        "invocation_id": call.evidence.invocation_id,
        "error": if call.executed { None } else { Some(first_line(&call.result_content)) },
    });
    let _ = app.emit("zylcode://tool-call", payload);
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use zylcode_core::providers::streaming::{
        ChatMessage, ChatRequest, FinishReason, ProviderError, StreamOutcome, ToolCall,
    };
    use zylcode_core::ModelProvider;

    /// A provider that asks for one tool and then answers, so the desktop path
    /// runs end to end without a network or a credential.
    struct ScriptedProvider {
        rounds: AtomicUsize,
        seen_tool_result: std::sync::Mutex<bool>,
    }

    impl ScriptedProvider {
        fn new() -> Self {
            Self {
                rounds: AtomicUsize::new(0),
                seen_tool_result: std::sync::Mutex::new(false),
            }
        }
    }

    #[async_trait::async_trait]
    impl zylcode_core::native_tools::ChatTurn for ScriptedProvider {
        async fn turn(
            &self,
            request: &ChatRequest,
            _cancel: &CancelToken,
        ) -> Result<StreamOutcome, ProviderError> {
            let round = self.rounds.fetch_add(1, Ordering::SeqCst);
            assert!(
                !request.tools.is_empty(),
                "the desktop path must declare tools"
            );

            if round == 0 {
                let mut outcome = StreamOutcome::empty(FinishReason::Stop);
                outcome.tool_calls = vec![ToolCall {
                    call_id: Some("call_desktop_1".to_string()),
                    block_id: None,
                    name: "fs_read".to_string(),
                    arguments: r#"{"path":"README.md"}"#.to_string(),
                    done: true,
                    metadata: None,
                }];
                return Ok(outcome);
            }

            // The continuation must carry the real tool content, otherwise the
            // loop only proves it can send a second request.
            let carried = request.messages.iter().any(|m| {
                matches!(
                    m,
                    ChatMessage::ToolResult { content, is_error, .. }
                        if !is_error && content.contains("DESKTOP-PROOF")
                )
            });
            *self.seen_tool_result.lock().unwrap() = carried;
            let mut outcome = StreamOutcome::empty(FinishReason::Stop);
            outcome.text = "README says DESKTOP-PROOF.".to_string();
            Ok(outcome)
        }
    }

    #[tokio::test]
    async fn desktop_session_runs_the_gated_tool_lifecycle() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("README.md"), "DESKTOP-PROOF\n").expect("fixture");
        let evidence = dir.path().join("evidence.jsonl");

        let turn = ScriptedProvider::new();

        let observed = Arc::new(AtomicUsize::new(0));
        let observed_in = observed.clone();

        let run = run_native_session(
            &turn,
            dir.path(),
            &evidence,
            "you are ZylCode",
            "read README.md and tell me what it says",
            "test-model",
            "desktop:chat",
            "desktop-test".to_string(),
            &CancelToken::new(),
            move |_call| {
                observed_in.fetch_add(1, Ordering::SeqCst);
            },
        )
        .await
        .expect("desktop session should complete");

        assert_eq!(run.rounds, 2, "one tool round, then the answer");
        assert_eq!(run.calls.len(), 1);
        let call = &run.calls[0];
        assert_eq!(call.wire_name, "fs_read");
        assert_eq!(call.tool_id, "fs.read");
        assert!(call.executed, "the restrictive gate allows reads");
        assert!(
            call.result_content.contains("DESKTOP-PROOF"),
            "the executor really read the file: {}",
            call.result_content
        );
        assert_eq!(
            observed.load(Ordering::SeqCst),
            1,
            "the UI observer saw the call"
        );
        assert_eq!(run.text, "README says DESKTOP-PROOF.");

        let ledger = std::fs::read_to_string(&evidence).expect("evidence written to app dir");
        assert!(ledger.contains("fs.read"), "ledger names the canonical id");
        assert!(ledger.contains("desktop:chat"), "ledger names the actor");

        // Nothing secret may reach the ledger.
        for needle in ["api_key", "API_KEY", "Bearer "] {
            assert!(!ledger.contains(needle), "ledger must not carry {needle}");
        }
    }

    #[tokio::test]
    async fn desktop_session_stops_when_cancelled() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("README.md"), "DESKTOP-PROOF\n").expect("fixture");
        let turn = ScriptedProvider::new();
        let cancel = CancelToken::new();
        cancel.cancel();

        let err = run_native_session(
            &turn,
            dir.path(),
            &dir.path().join("evidence.jsonl"),
            "",
            "read README.md",
            "test-model",
            "desktop:chat",
            "desktop-test".to_string(),
            &cancel,
            |_| {},
        )
        .await
        .expect_err("a cancelled turn must not answer");

        assert_eq!(err, NativeToolError::Cancelled);
    }

    #[test]
    fn capability_flag_decides_whether_tools_are_declared() {
        // The desktop gate reads the capability table, it does not assume.
        assert!(provider_supports_native_tools(
            &ModelProvider::OpenAiCompatible
        ));
        assert!(!provider_supports_native_tools(
            &ModelProvider::SyntheticOffline
        ));
    }

    #[test]
    fn cancel_slot_is_idempotent() {
        let slot = CancelSlot::default();
        assert!(!slot.cancel(), "nothing to cancel before a turn starts");
        let token = CancelToken::new();
        slot.install(token.clone());
        assert!(slot.cancel());
        assert!(token.is_cancelled());
        slot.clear();
        assert!(!slot.cancel());
    }
}
