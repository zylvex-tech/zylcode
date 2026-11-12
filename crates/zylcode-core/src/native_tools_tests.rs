//! Tests for the native tool loop.
//!
//! Every case runs the real pipeline: real `dispatch`, real gate, real evidence
//! sink, real tool executors on a temporary workspace. Only the provider is
//! scripted, because a protocol test does not need a network and a network test
//! cannot be deterministic.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::native_tools::{
    resolve, tool_declarations, ChatTurn, NativeToolError, NativeToolRunner, MAX_TOOL_RESULT_CHARS,
};
use crate::providers::streaming::{
    wire_name, CancelToken, ChatMessage, ChatRequest, FinishReason, ProviderError, StreamOutcome,
    ToolCall, ToolSpec,
};
use zylcode_mcp::evidence::{JsonlEvidenceSink, ToolRuntime};
use zylcode_mcp::permission::{PermissionGate, PermissionPolicy};
use zylcode_mcp::real_tools::ToolContext;

/// A provider whose answers are fixed in advance, recording what it was asked.
struct Script {
    rounds: Mutex<VecDeque<std::result::Result<StreamOutcome, ProviderError>>>,
    requests: Mutex<Vec<ChatRequest>>,
}

impl Script {
    fn new(rounds: Vec<std::result::Result<StreamOutcome, ProviderError>>) -> Self {
        Self {
            rounds: Mutex::new(rounds.into()),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn text(text: &str) -> std::result::Result<StreamOutcome, ProviderError> {
        Ok(outcome(text, Vec::new()))
    }

    fn calls(calls: Vec<ToolCall>) -> std::result::Result<StreamOutcome, ProviderError> {
        Ok(outcome("", calls))
    }

    fn requests(&self) -> Vec<ChatRequest> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl ChatTurn for Script {
    async fn turn(
        &self,
        request: &ChatRequest,
        _cancel: &CancelToken,
    ) -> std::result::Result<StreamOutcome, ProviderError> {
        self.requests.lock().unwrap().push(request.clone());
        self.rounds
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Ok(outcome("done", Vec::new())))
    }
}

fn outcome(text: &str, tool_calls: Vec<ToolCall>) -> StreamOutcome {
    StreamOutcome {
        text: text.to_string(),
        finish: FinishReason::Stop,
        usage: None,
        chunks: 1,
        provider_ref: None,
        tool_calls,
    }
}

fn call(name: &str, arguments: &str, id: &str) -> ToolCall {
    ToolCall {
        call_id: Some(id.to_string()),
        block_id: None,
        name: name.to_string(),
        arguments: arguments.to_string(),
        done: true,
        metadata: None,
    }
}

fn spec(id: &str) -> ToolSpec {
    ToolSpec {
        name: id.to_string(),
        description: "test tool".to_string(),
        parameters: serde_json::json!({ "type": "object", "properties": {} }),
    }
}

/// A scratch workspace with one file, removed on drop.
struct Workspace {
    path: std::path::PathBuf,
}

impl Workspace {
    fn new(label: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("zylcode-native-{label}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("workspace");
        Self { path }
    }

    fn ledger(&self) -> std::path::PathBuf {
        self.path.join("evidence.jsonl")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn context(workspace: &Workspace) -> ToolContext {
    ToolContext {
        working_directory: workspace.path.clone(),
        environment: Default::default(),
        timeout: std::time::Duration::from_secs(10),
        session_id: Some("native-test".to_string()),
        actor: Some("agent:native-test".to_string()),
        approval_required: false,
    }
}

fn runtime(workspace: &Workspace) -> ToolRuntime {
    ToolRuntime::default().with_evidence_at(workspace.ledger())
}

fn evidence(workspace: &Workspace) -> Vec<zylcode_mcp::real_tools::ToolEvidence> {
    JsonlEvidenceSink::new(workspace.ledger())
        .read_all()
        .unwrap_or_default()
}

// -- the mapping -----------------------------------------------------------

#[test]
fn dotted_ids_are_mapped_but_keep_their_canonical_identity() {
    assert_eq!(wire_name("fs.read"), "fs_read");
    assert_eq!(wire_name("search.grep"), "search_grep");
    assert_eq!(wire_name("already_safe"), "already_safe");
    // The declaration keeps the dotted id; only the wire copy is sanitised.
    assert_eq!(spec("fs.read").name, "fs.read");
    assert_eq!(spec("fs.read").wire_name(), "fs_read");
}

#[test]
fn resolution_accepts_the_wire_name_and_refuses_an_undeclared_one() {
    let tools = vec![spec("fs.read")];
    assert_eq!(resolve(&tools, "fs_read").unwrap().name, "fs.read");
    assert_eq!(resolve(&tools, "fs.read").unwrap().name, "fs.read");
    assert!(resolve(&tools, "fs_delete_everything").is_none());
}

#[test]
fn declared_tools_are_all_executable_and_unambiguous() {
    let tools = tool_declarations().expect("declarations");
    assert!(
        !tools.is_empty(),
        "the catalogue must offer executable tools"
    );
    let mut wires = Vec::new();
    for tool in &tools {
        assert!(
            zylcode_mcp::real_tools::get_real_tool(&tool.name).is_some(),
            "{} was declared without an executor",
            tool.name
        );
        let wire = tool.wire_name();
        assert!(!wires.contains(&wire), "duplicate wire name {wire}");
        wires.push(wire);
    }
}

// -- the round trip --------------------------------------------------------

#[tokio::test]
async fn a_declared_call_executes_records_evidence_and_continues() {
    let workspace = Workspace::new("roundtrip");
    std::fs::write(workspace.path.join("note.txt"), "hello from the workspace").unwrap();
    let script = Script::new(vec![
        Script::calls(vec![call("fs_read", r#"{"path":"note.txt"}"#, "call_1")]),
        Script::text("the file says hello"),
    ]);

    let run = NativeToolRunner::new(&script, &runtime(&workspace), context(&workspace))
        .run("read note.txt", "test-model", &CancelToken::new())
        .await
        .expect("run completes");

    assert_eq!(run.text, "the file says hello");
    assert_eq!(run.rounds, 2);
    assert_eq!(run.calls.len(), 1);
    let executed = &run.calls[0];
    assert_eq!(executed.wire_name, "fs_read");
    assert_eq!(
        executed.tool_id, "fs.read",
        "execution identity is the canonical dotted id"
    );
    assert!(executed.executed);
    assert!(executed.result_content.contains("hello from the workspace"));

    // Evidence exists for the invocation, attributed to this run.
    let records = evidence(&workspace);
    assert_eq!(records.len(), 1, "one record for one invocation");
    assert_eq!(records[0].tool_id, "fs.read");
    assert_eq!(records[0].actor.as_deref(), Some("agent:native-test"));
    assert!(records[0].approval_decision.is_some());
    assert!(records[0].end_time.is_some(), "a record must be placeable");

    // The second request carries the assistant turn and the tool result, so the
    // model can see what it asked for and what came back.
    let requests = script.requests();
    assert_eq!(requests.len(), 2);
    assert!(!requests[0].tools.is_empty(), "round 1 declares tools");
    let turns = requests[1].turns();
    assert!(matches!(turns[1], ChatMessage::Assistant { .. }));
    match &turns[2] {
        ChatMessage::ToolResult {
            call_id,
            name,
            content,
            is_error,
        } => {
            assert_eq!(call_id, "call_1");
            assert_eq!(name, "fs.read");
            assert!(!is_error);
            assert!(content.contains("hello from the workspace"));
        }
        other => panic!("expected a tool result turn, got {other:?}"),
    }
}

#[tokio::test]
async fn several_calls_in_one_round_are_all_executed() {
    let workspace = Workspace::new("multi");
    std::fs::write(workspace.path.join("a.txt"), "alpha").unwrap();
    std::fs::write(workspace.path.join("b.txt"), "beta").unwrap();
    let script = Script::new(vec![
        Script::calls(vec![
            call("fs_read", r#"{"path":"a.txt"}"#, "call_a"),
            call("fs_read", r#"{"path":"b.txt"}"#, "call_b"),
        ]),
        Script::text("read both"),
    ]);

    let run = NativeToolRunner::new(&script, &runtime(&workspace), context(&workspace))
        .run("read both", "test-model", &CancelToken::new())
        .await
        .expect("run completes");

    assert_eq!(run.calls.len(), 2);
    assert!(run.calls.iter().all(|c| c.executed));
    assert_eq!(evidence(&workspace).len(), 2);
    let turns = script.requests()[1].turns();
    assert_eq!(
        turns
            .iter()
            .filter(|t| matches!(t, ChatMessage::ToolResult { .. }))
            .count(),
        2
    );
}

// -- fail closed -----------------------------------------------------------

#[tokio::test]
async fn an_undeclared_tool_is_refused_without_executing_anything() {
    let workspace = Workspace::new("unknown");
    let script = Script::new(vec![Script::calls(vec![call(
        "fs_delete",
        r#"{"path":"a.txt"}"#,
        "call_x",
    )])]);

    let error = NativeToolRunner::new(&script, &runtime(&workspace), context(&workspace))
        .run("delete a.txt", "test-model", &CancelToken::new())
        .await
        .expect_err("an undeclared tool must stop the run");

    assert_eq!(
        error,
        NativeToolError::UnknownTool {
            wire_name: "fs_delete".to_string()
        }
    );
    assert!(
        evidence(&workspace).is_empty(),
        "nothing was dispatched, so nothing was recorded"
    );
    assert_eq!(
        script.requests().len(),
        1,
        "the run stops rather than continuing on a refusal"
    );
}

#[tokio::test]
async fn malformed_arguments_are_refused_before_the_gate() {
    let workspace = Workspace::new("malformed");
    for arguments in ["not json", r#"["a.txt"]"#, "\"a.txt\""] {
        let script = Script::new(vec![Script::calls(vec![call(
            "fs_read", arguments, "call_bad",
        )])]);
        let error = NativeToolRunner::new(&script, &runtime(&workspace), context(&workspace))
            .run("read", "test-model", &CancelToken::new())
            .await
            .expect_err("malformed arguments must stop the run");
        assert!(
            matches!(error, NativeToolError::MalformedCall { .. }),
            "{arguments:?} produced {error:?}"
        );
    }
    assert!(evidence(&workspace).is_empty());
}

#[tokio::test]
async fn a_call_without_an_id_cannot_be_answered_and_is_refused() {
    let workspace = Workspace::new("no-id");
    let mut anonymous = call("fs_read", r#"{"path":"a.txt"}"#, "unused");
    anonymous.call_id = None;
    let script = Script::new(vec![Script::calls(vec![anonymous])]);

    let error = NativeToolRunner::new(&script, &runtime(&workspace), context(&workspace))
        .run("read", "test-model", &CancelToken::new())
        .await
        .expect_err("an unaddressable call cannot be answered");

    assert!(matches!(error, NativeToolError::MalformedCall { .. }));
}

#[tokio::test]
async fn a_denied_tool_is_recorded_and_reported_to_the_model() {
    let workspace = Workspace::new("denied");
    // The restrictive policy denies anything above `Read`, so a write is refused.
    let gate = Arc::new(PermissionGate::new(PermissionPolicy::restrictive()));
    let runtime = runtime(&workspace).with_gate(gate);
    let script = Script::new(vec![
        Script::calls(vec![call(
            "fs_write",
            r#"{"path":"new.txt","content":"x"}"#,
            "call_w",
        )]),
        Script::text("I could not write the file"),
    ]);

    let run = NativeToolRunner::new(&script, &runtime, context(&workspace))
        .run("write new.txt", "test-model", &CancelToken::new())
        .await
        .expect("a refusal is information, not a crash");

    assert_eq!(run.calls.len(), 1);
    assert!(!run.calls[0].executed, "the gate refused");
    assert!(
        run.calls[0].decision.contains("deny")
            || run.calls[0].decision.contains("require_approval")
    );
    assert!(run.calls[0].result_content.contains("tool error"));

    let records = evidence(&workspace);
    assert_eq!(records.len(), 1, "a refusal is evidence");
    assert_eq!(records[0].tool_id, "fs.write");
    assert!(records[0].approval_decision.is_some());
    assert!(
        !workspace.path.join("new.txt").exists(),
        "the refused write must not have happened"
    );

    // The model was told, and the run continued to a final answer.
    match &script.requests()[1].turns()[2] {
        ChatMessage::ToolResult { is_error, .. } => assert!(is_error),
        other => panic!("expected an error tool result, got {other:?}"),
    }
}

#[tokio::test]
async fn the_round_limit_stops_a_model_that_never_finishes() {
    let workspace = Workspace::new("rounds");
    std::fs::write(workspace.path.join("a.txt"), "alpha").unwrap();
    // Always asks for the same tool, never concludes.
    let script = Script::new(
        (0..10)
            .map(|i| {
                Script::calls(vec![call(
                    "fs_read",
                    r#"{"path":"a.txt"}"#,
                    &format!("c{i}"),
                )])
            })
            .collect(),
    );

    let error = NativeToolRunner::new(&script, &runtime(&workspace), context(&workspace))
        .with_max_rounds(3)
        .run("loop forever", "test-model", &CancelToken::new())
        .await
        .expect_err("an unfinished loop must stop");

    assert_eq!(error, NativeToolError::RoundLimit(3));
    assert_eq!(script.requests().len(), 3);
}

#[tokio::test]
async fn a_cancelled_run_never_reaches_the_provider() {
    let workspace = Workspace::new("cancel");
    let script = Script::new(vec![Script::text("unreachable")]);
    let cancel = CancelToken::new();
    cancel.cancel();

    let error = NativeToolRunner::new(&script, &runtime(&workspace), context(&workspace))
        .run("anything", "test-model", &cancel)
        .await
        .expect_err("cancelled");

    assert_eq!(error, NativeToolError::Cancelled);
    assert!(script.requests().is_empty());
}

#[tokio::test]
async fn a_provider_failure_is_reported_as_such() {
    let workspace = Workspace::new("provider-error");
    let script = Script::new(vec![Err(ProviderError {
        kind: crate::providers::streaming::ProviderErrorKind::Authentication,
        message: "401 from provider".to_string(),
        code: Some("401".to_string()),
        raw: None,
    })]);

    let error = NativeToolRunner::new(&script, &runtime(&workspace), context(&workspace))
        .run("anything", "test-model", &CancelToken::new())
        .await
        .expect_err("provider failure");

    assert!(matches!(error, NativeToolError::Provider(_)));
    assert!(error.to_string().contains("401"));
}

#[test]
fn oversized_tool_output_is_truncated_and_says_so() {
    let limit = MAX_TOOL_RESULT_CHARS;
    let content = "x".repeat(limit + 10);
    let rendered = super::truncate(&content, limit);
    assert!(rendered.contains("truncated at"));
    assert!(rendered.chars().count() < content.chars().count() + 120);
}
