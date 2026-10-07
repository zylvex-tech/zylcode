//! From an N-ATLAS response to a bounded, validated ZylCode task graph.
//!
//! This is the handoff: the point where model output stops being prose and
//! becomes a structure the factory can execute and verify.
//!
//! # The schema is ours
//!
//! The JSON parsed here is a contract **ZylCode defines and asks for** in the
//! system preamble. It is not a claim about N-ATLAS's response format, and it
//! is deliberately strict: a response that does not satisfy it is rejected with
//! [`NatlasError::MalformedResponse`] rather than partially accepted.
//!
//! # What the handoff does *not* invent
//!
//! A step that would need real content — the body of a patch, the exact test
//! command — is mapped to a side-effect-free [`TaskAction::Note`] **unless the
//! model actually supplied it** in the contract. A `kind: implement` step whose
//! `content`/`path` are absent is recorded as a note; we do not fabricate file
//! bodies. The one step that never needs invented content is
//! [`IntentStepKind::Approve`], which maps to a **real** [`TaskAction::Approval`]
//! gate: a human decision needs no invented content, so it is executed for real.
//!
//! # Approval gates every real mutation (structurally)
//!
//! A [`TaskAction::WriteFile`], [`TaskAction::RunCommand`] or [`TaskAction::Verify`]
//! is *only* emitted when the model supplied the concrete `content`/`path` or
//! `command`. Every such side-effect node is made to depend on the first
//! [`IntentStepKind::Approve`] node, so the graph cannot execute a mutation
//! before a human approves the plan — regardless of the order the model listed
//! the steps in. This is the competition's "no mutation before approval" rule,
//! enforced by the DAG, not by hope.

use serde::{Deserialize, Serialize};

use super::types::NatlasError;
use crate::factory::graph::{TaskAction, TaskGraph, TaskKind, TaskNode};

/// The kind of work a step describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentStepKind {
    Analyse,
    Implement,
    Test,
    Document,
    /// Run an arbitrary command (build, execute). Maps to [`TaskAction::RunCommand`]
    /// when the model supplied a `command`.
    Run,
    Approve,
}

impl IntentStepKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "analyse" | "analyze" => Some(Self::Analyse),
            "implement" => Some(Self::Implement),
            "test" => Some(Self::Test),
            "document" => Some(Self::Document),
            "run" => Some(Self::Run),
            "approve" => Some(Self::Approve),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Analyse => "analyse",
            Self::Implement => "implement",
            Self::Test => "test",
            Self::Document => "document",
            Self::Run => "run",
            Self::Approve => "approve",
        }
    }
}

/// One step of a structured engineering intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatlasIntentStep {
    pub kind: IntentStepKind,
    pub description: String,
    /// Legacy/optional free-form path hint (not used for the write target).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
    /// File path for an `implement`/`document` step that carries real content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Real file content. Absent ⇒ the step is recorded as a [`TaskAction::Note`]
    /// (no fabricated content).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// `argv` for a `test`/`run` step. Present ⇒ the step becomes a real command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    /// For a `test` step: when true the command is a verification
    /// ([`TaskAction::Verify`]); otherwise a plain [`TaskAction::RunCommand`].
    #[serde(default)]
    pub verify: bool,
}

/// A structured engineering intent derived from an N-ATLAS response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatlasEngineeringIntent {
    pub summary: String,
    pub steps: Vec<NatlasIntentStep>,
}

impl NatlasEngineeringIntent {
    /// Parse the ZylCode intent contract out of a model response.
    ///
    /// Accepts the JSON object either bare or wrapped in a ```json fence, which
    /// is how models habitually return structured data. Everything else is
    /// rejected with a precise reason.
    pub fn parse(text: &str) -> Result<Self, NatlasError> {
        let json = strip_code_fence(text);
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| NatlasError::MalformedResponse {
                detail: format!("intent is not valid JSON: {e}"),
            })?;
        Self::from_value(value)
    }

    fn from_value(value: serde_json::Value) -> Result<Self, NatlasError> {
        let obj = value.as_object().ok_or_else(|| NatlasError::MalformedResponse {
            detail: "intent must be a JSON object".to_string(),
        })?;

        let summary = obj
            .get("summary")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| NatlasError::MalformedResponse {
                detail: "intent requires a non-empty `summary`".to_string(),
            })?
            .to_string();

        let steps_value = obj
            .get("steps")
            .and_then(|v| v.as_array())
            .ok_or_else(|| NatlasError::MalformedResponse {
                detail: "intent requires a `steps` array".to_string(),
            })?;
        if steps_value.is_empty() {
            return Err(NatlasError::MalformedResponse {
                detail: "intent `steps` must not be empty".to_string(),
            });
        }

        let mut steps = Vec::with_capacity(steps_value.len());
        for (i, raw) in steps_value.iter().enumerate() {
            let obj_i = raw
                .as_object()
                .ok_or_else(|| NatlasError::MalformedResponse {
                    detail: format!("step {i} must be a JSON object"),
                })?;
            let kind_str = obj_i
                .get("kind")
                .and_then(|v| v.as_str())
                .ok_or_else(|| NatlasError::MalformedResponse {
                    detail: format!("step {i} is missing a string `kind`"),
                })?;
            let kind = IntentStepKind::parse(kind_str).ok_or_else(|| {
                NatlasError::MalformedResponse {
                    detail: format!(
                        "step {i} has unknown kind `{kind_str}`; expected one of \
                         analyse|implement|test|document|run|approve"
                    ),
                }
            })?;
            let description = obj_i
                .get("description")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| NatlasError::MalformedResponse {
                    detail: format!("step {i} is missing a non-empty `description`"),
                })?
                .to_string();

            let target_path = obj_i
                .get("target_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let path = obj_i
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let content = obj_i
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let command = obj_i
                .get("command")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .filter(|c| !c.is_empty());
            let verify = obj_i
                .get("verify")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            steps.push(NatlasIntentStep {
                kind,
                description,
                target_path,
                path,
                content,
                command,
                verify,
            });
        }

        Ok(Self { summary, steps })
    }

    /// Convert the intent into a validated, dependency-chained factory task
    /// graph.
    ///
    /// The graph is what the existing `FactoryRunner` consumes, so this is the
    /// seam between the competition work and the product's durable workflow.
    ///
    /// Approval gating is structural: every real side-effect node
    /// (WriteFile / RunCommand / Verify) depends on the first `approve` step,
    /// so a mutation can never execute before a human approves — no matter how
    /// the model ordered the steps.
    pub fn to_task_graph(&self) -> Result<TaskGraph, NatlasError> {
        // The approval gate id, computed upfront so a side-effect node can depend
        // on it no matter where in the step list the approval appears — a mutation
        // listed *before* the approval is still correctly gated on it.
        let approval_id = self.approval_step_id();

        let mut graph = TaskGraph::new();
        let mut note_prev: Option<String> = None;
        let mut effect_prev: Option<String> = None;

        for (i, step) in self.steps.iter().enumerate() {
            let id = format!("step-{:02}", i + 1);

            let (task_kind, action) = match step.kind {
                // Side-effect-free: recorded, nothing executed.
                IntentStepKind::Analyse => (
                    TaskKind::RepositoryContext,
                    TaskAction::Note {
                        text: format!("analyse: {}", step.description),
                    },
                ),
                // A real file write, only when content + path were supplied.
                IntentStepKind::Implement | IntentStepKind::Document => {
                    if let (Some(path), Some(content)) = (&step.path, &step.content) {
                        let kind = if step.kind == IntentStepKind::Document {
                            TaskKind::Documentation
                        } else {
                            TaskKind::Implementation
                        };
                        (
                            kind,
                            TaskAction::WriteFile {
                                path: path.clone(),
                                content: content.clone(),
                            },
                        )
                    } else {
                        let (kind, label) = if step.kind == IntentStepKind::Document {
                            (TaskKind::Documentation, "document")
                        } else {
                            (TaskKind::Implementation, "implement")
                        };
                        (
                            kind,
                            TaskAction::Note {
                                text: format!("{label} (content not supplied): {}", step.description),
                            },
                        )
                    }
                }
                // A verification command (test) or a plain command (run/build).
                IntentStepKind::Test | IntentStepKind::Run => {
                    match &step.command {
                        Some(argv) if !argv.is_empty() => {
                            let command = argv[0].clone();
                            let args = argv[1..].to_vec();
                            match step.kind {
                                IntentStepKind::Test if step.verify => (
                                    TaskKind::Test,
                                    TaskAction::Verify { command, args },
                                ),
                                _ => (
                                    if step.kind == IntentStepKind::Test {
                                        TaskKind::Test
                                    } else {
                                        TaskKind::Implementation
                                    },
                                    TaskAction::RunCommand {
                                        command,
                                        args,
                                        expect_exit: 0,
                                    },
                                ),
                            }
                        }
                        _ => {
                            let (kind, label) = if step.kind == IntentStepKind::Test {
                                (TaskKind::Test, "test")
                            } else {
                                (TaskKind::Implementation, "run")
                            };
                            (
                                kind,
                                TaskAction::Note {
                                    text: format!("{label} (command not supplied): {}", step.description),
                                },
                            )
                        }
                    }
                }
                // A real gate: no invented content is required, so this action
                // genuinely parks the job until a human approves.
                IntentStepKind::Approve => (
                    TaskKind::HumanApproval,
                    TaskAction::Approval {
                        reason: step.description.clone(),
                    },
                ),
            };

            // Decide dependencies.
            let mut depends: Vec<String> = match &action {
                TaskAction::Approval { .. } => note_prev.clone().into_iter().collect(),
                TaskAction::WriteFile { .. }
                | TaskAction::RunCommand { .. }
                | TaskAction::Verify { .. } => {
                    if let Some(a) = &approval_id {
                        vec![a.clone()]
                    } else if let Some(p) = &effect_prev {
                        vec![p.clone()]
                    } else {
                        note_prev.clone().into_iter().collect()
                    }
                }
                TaskAction::Note { .. } => note_prev.clone().into_iter().collect(),
                _ => Vec::new(),
            };
            depends.retain(|d| d != &id);
            depends.sort();
            depends.dedup();

            let mut node = TaskNode::new(id.clone(), task_kind, step.description.clone(), action);
            if !depends.is_empty() {
                node = node.depends_on(&depends.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            }
            graph
                .add_node(node)
                .map_err(|e| NatlasError::MalformedResponse {
                    detail: format!("task graph rejected step {id}: {e}"),
                })?;

            match graph.get(&id).unwrap().action {
                TaskAction::WriteFile { .. }
                | TaskAction::RunCommand { .. }
                | TaskAction::Verify { .. } => {
                    effect_prev = Some(id.clone());
                }
                TaskAction::Approval { .. } => {
                    // The approval gate does not advance the note/effect chain;
                    // later side effects still gate on it via `approval_id`.
                }
                _ => {
                    note_prev = Some(id.clone());
                }
            }
        }

        graph.validate().map_err(|e| NatlasError::MalformedResponse {
            detail: format!("intent produced an invalid task graph: {e}"),
        })?;
        Ok(graph)
    }

    /// The id of the first approval step, if the intent contains one. Exposed so
    /// callers (and tests) can assert that side effects are gated on it.
    pub fn approval_step_id(&self) -> Option<String> {
        self.steps
            .iter()
            .position(|s| s.kind == IntentStepKind::Approve)
            .map(|i| format!("step-{:02}", i + 1))
    }

    /// Parse the ZylCode intent contract, with a bounded repair attempt on the
    /// first strict-JSON failure.
    ///
    /// The repair layer handles common LLM serialization defects (triple-quoted
    /// strings, raw newlines inside strings, trailing commas) without changing
    /// semantic content. If repair succeeds, `was_repaired` is set to `true` so
    /// the caller can record that normalization occurred.
    pub fn parse_with_repair(text: &str) -> Result<(Self, bool), NatlasError> {
        let json = strip_code_fence(text);
        match serde_json::from_str::<serde_json::Value>(json) {
            Ok(value) => Self::from_value(value).map(|i| (i, false)),
            Err(_) => {
                if let Some(repaired) = repair_json(json) {
                    let value: serde_json::Value = serde_json::from_str(&repaired).map_err(|e| {
                        NatlasError::MalformedResponse {
                            detail: format!(
                                "intent is not valid JSON even after repair: {e}"
                            ),
                        }
                    })?;
                    Self::from_value(value).map(|i| (i, true))
                } else {
                    Err(NatlasError::MalformedResponse {
                        detail: "intent is not valid JSON and no repair applied".to_string(),
                    })
                }
            }
        }
    }
}

/// Strip a leading ```json / ``` fence and its closing fence, if present.
fn strip_code_fence(text: &str) -> &str {
    let trimmed = text.trim();
    if !trimmed.starts_with("```") {
        return trimmed;
    }
    let after_open = match trimmed.find('\n') {
        Some(nl) => &trimmed[nl + 1..],
        None => return trimmed,
    };
    match after_open.rfind("```") {
        Some(end) => after_open[..end].trim(),
        None => after_open.trim(),
    }
}

/// Bounded, deterministic repair for common LLM JSON serialization defects.
///
/// Models habitually emit malformed JSON: triple-quoted strings with raw
/// newlines, unescaped quotes inside string values, trailing commas, etc.
/// This repair is **narrowly scoped** to defects that do not change semantic
/// content. It never invents missing fields, never changes paths/commands,
/// and records whether it touched the text so callers can audit the boundary.
///
/// Returns `Some(repaired)` if the text was modified, `None` if no repair
/// was needed or if the defect is outside the bounded scope.
pub fn repair_json(text: &str) -> Option<String> {
    let mut repaired = text.to_string();
    let mut changed = false;

    // 1. Triple-quoted strings: """...""" → "..." with newlines escaped.
    // This is the EV-017 defect: the model emitted a Python triple-quoted
    // string literal inside what it claimed was JSON.
    loop {
        if let Some(start) = repaired.find("\"\"\"") {
            if let Some(end) = repaired[start + 3..].find("\"\"\"") {
                let end_abs = start + 3 + end;
                let inner = &repaired[start + 3..end_abs];
                // Escape backslashes first, then quotes, then newlines.
                let escaped = inner
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r");
                repaired.replace_range(start..end_abs + 3, &format!("\"{escaped}\""));
                changed = true;
                continue;
            }
        }
        break;
    }

    // 2. Raw newlines inside regular double-quoted strings (not triple-quoted).
    // Scan for quote pairs that contain unescaped newlines and escape them.
    // This is a best-effort pass; if it can't pair quotes safely it stops.
    let mut result = String::with_capacity(repaired.len());
    let bytes = repaired.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            // Find the closing quote, respecting escapes.
            result.push('"');
            i += 1;
            let mut escaped = false;
            while i < bytes.len() {
                let c = bytes[i];
                if escaped {
                    result.push(c as char);
                    escaped = false;
                    i += 1;
                    continue;
                }
                if c == b'\\' {
                    result.push('\\');
                    escaped = true;
                    i += 1;
                    continue;
                }
                if c == b'"' {
                    result.push('"');
                    i += 1;
                    break;
                }
                if c == b'\n' {
                    result.push_str("\\n");
                    changed = true;
                    i += 1;
                    continue;
                }
                if c == b'\r' {
                    result.push_str("\\r");
                    changed = true;
                    i += 1;
                    continue;
                }
                result.push(c as char);
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    repaired = result;

    // 3. Trailing commas before `]` or `}` — common LLM habit.
    repaired = repaired.replace(",]", "]").replace(",}", "}");
    if repaired != text {
        changed = true;
    }

    if changed {
        Some(repaired)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{
        "summary": "Add a health endpoint",
        "steps": [
            {"kind": "analyse", "description": "find the router"},
            {"kind": "implement", "description": "add GET /health", "target_path": "src/routes.rs"},
            {"kind": "test", "description": "run the suite"},
            {"kind": "document", "description": "note the endpoint"},
            {"kind": "approve", "description": "supervisor sign-off"}
        ]
    }"#;

    #[test]
    fn parses_a_well_formed_intent() {
        let intent = NatlasEngineeringIntent::parse(GOOD).unwrap();
        assert_eq!(intent.steps.len(), 5);
        assert_eq!(intent.steps[1].kind, IntentStepKind::Implement);
        assert_eq!(intent.steps[1].target_path.as_deref(), Some("src/routes.rs"));
    }

    #[test]
    fn parses_an_intent_wrapped_in_a_code_fence() {
        let fenced = format!("```json\n{GOOD}\n```");
        assert!(NatlasEngineeringIntent::parse(&fenced).is_ok());
    }

    #[test]
    fn parses_optional_content_and_command_fields() {
        let json = r#"{
            "summary": "build a counter",
            "steps": [
                {"kind": "implement", "description": "write the module",
                 "path": "lib.rs", "content": "pub fn x() {}\n"},
                {"kind": "test", "description": "run the suite",
                 "command": ["cargo", "test"], "verify": true},
                {"kind": "run", "description": "format the code",
                 "command": ["cargo", "fmt"]}
            ]
        }"#;
        let intent = NatlasEngineeringIntent::parse(json).unwrap();
        assert_eq!(intent.steps[0].path.as_deref(), Some("lib.rs"));
        assert_eq!(intent.steps[0].content.as_deref(), Some("pub fn x() {}\n"));
        assert_eq!(intent.steps[1].command.as_ref().unwrap(), &vec!["cargo", "test"]);
        assert!(intent.steps[1].verify);
        assert!(matches!(intent.steps[2].kind, IntentStepKind::Run));
    }

    #[test]
    fn rejects_unknown_step_kinds() {
        let bad = r#"{"summary":"s","steps":[{"kind":"deploy","description":"x"}]}"#;
        let err = NatlasEngineeringIntent::parse(bad).unwrap_err();
        assert!(err.to_string().contains("unknown kind"));
    }

    #[test]
    fn rejects_empty_steps() {
        let bad = r#"{"summary":"s","steps":[]}"#;
        assert_eq!(
            NatlasEngineeringIntent::parse(bad).unwrap_err().code(),
            "MALFORMED_RESPONSE"
        );
    }

    #[test]
    fn handoff_produces_a_valid_dependency_chained_graph() {
        let intent = NatlasEngineeringIntent::parse(GOOD).unwrap();
        let graph = intent.to_task_graph().unwrap();
        // Five steps, each depending on the previous one.
        let summary = graph.summary();
        assert_eq!(summary.total, 5);
        assert_eq!(summary.pending, 5, "a fresh graph has every task pending");
        assert_eq!(
            graph.ready(),
            vec!["step-01".to_string()],
            "only the first step is dependency-satisfied"
        );
        // The approval step is a real gate, not an annotation.
        assert_eq!(graph.get("step-05").unwrap().kind, TaskKind::HumanApproval);
    }

    #[test]
    fn handoff_graph_is_rejected_if_a_step_is_missing_a_description() {
        let bad = r#"{"summary":"s","steps":[{"kind":"analyse"}]}"#;
        assert!(NatlasEngineeringIntent::parse(bad).is_err());
    }

    #[test]
    fn content_maps_to_a_real_writefile_action() {
        let json = r#"{
            "summary": "add a module",
            "steps": [
                {"kind": "approve", "description": "owner sign-off"},
                {"kind": "implement", "description": "write the file",
                 "path": "lib.rs", "content": "pub fn value() -> u32 { 42 }\n"}
            ]
        }"#;
        let graph = NatlasEngineeringIntent::parse(json).unwrap().to_task_graph().unwrap();
        let write = graph.get("step-02").unwrap();
        match &write.action {
            TaskAction::WriteFile { path, content } => {
                assert_eq!(path, "lib.rs");
                assert!(content.contains("fn value"));
            }
            other => panic!("expected WriteFile, got {other:?}"),
        }
    }

    #[test]
    fn test_step_with_verify_maps_to_verify_action() {
        let json = r#"{
            "summary": "verify the suite",
            "steps": [
                {"kind": "approve", "description": "owner sign-off"},
                {"kind": "test", "description": "run the suite",
                 "command": ["cargo", "test"], "verify": true}
            ]
        }"#;
        let graph = NatlasEngineeringIntent::parse(json).unwrap().to_task_graph().unwrap();
        assert!(matches!(
            graph.get("step-02").unwrap().action,
            TaskAction::Verify { .. }
        ));
    }

    #[test]
    fn run_step_maps_to_runcommand_action() {
        let json = r#"{
            "summary": "format",
            "steps": [
                {"kind": "approve", "description": "owner sign-off"},
                {"kind": "run", "description": "format the code",
                 "command": ["cargo", "fmt"]}
            ]
        }"#;
        let graph = NatlasEngineeringIntent::parse(json).unwrap().to_task_graph().unwrap();
        let run = graph.get("step-02").unwrap();
        assert!(matches!(run.action, TaskAction::RunCommand { .. }));
        if let TaskAction::RunCommand { command, args, expect_exit } = &run.action {
            assert_eq!(command, "cargo");
            assert_eq!(args, &vec!["fmt"]);
            assert_eq!(*expect_exit, 0);
        }
    }

    #[test]
    fn missing_content_falls_back_to_note_not_a_fabricated_write() {
        // An implement step with no content must NOT invent a file body.
        let json = r#"{
            "summary": "add a module",
            "steps": [
                {"kind": "approve", "description": "owner sign-off"},
                {"kind": "implement", "description": "write the file"}
            ]
        }"#;
        let graph = NatlasEngineeringIntent::parse(json).unwrap().to_task_graph().unwrap();
        assert!(matches!(
            graph.get("step-02").unwrap().action,
            TaskAction::Note { .. }
        ));
    }

    #[test]
    fn approval_gates_every_side_effect_regardless_of_order() {
        // Even when the model lists the mutation BEFORE the approval, the side
        // effect must depend on the approval node — no mutation before approval.
        let json = r#"{
            "summary": "add a module",
            "steps": [
                {"kind": "implement", "description": "write the file",
                 "path": "lib.rs", "content": "x\n"},
                {"kind": "approve", "description": "owner sign-off"}
            ]
        }"#;
        let intent = NatlasEngineeringIntent::parse(json).unwrap();
        let graph = intent.to_task_graph().unwrap();
        let approval = intent.approval_step_id().expect("has approval");
        let write = graph.get("step-01").unwrap();
        assert!(
            write.depends_on.contains(&approval),
            "mutation must depend on approval even when listed first: {write:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Bounded JSON repair (EV-017 follow-up)
    // -----------------------------------------------------------------------

    #[test]
    fn repair_escapes_raw_newlines_inside_triple_quoted_strings() {
        // The EV-017 defect: model emitted """...""" with raw newlines.
        let bad = r#"{"summary":"s","steps":[{"kind":"implement","description":"d","path":"f.py","content":"""line1
line2"""}]}"#;
        let repaired = repair_json(bad).expect("repair should succeed");
        assert!(repaired.contains("\\n"), "newlines must be escaped");
        assert!(!repaired.contains("\"\"\""), "triple quotes must be removed");
        // Must now parse successfully.
        let intent = NatlasEngineeringIntent::parse(&repaired).unwrap();
        assert_eq!(intent.steps[0].content.as_deref(), Some("line1\nline2"));
    }

    #[test]
    fn repair_removes_trailing_commas() {
        let bad = r#"{"summary":"s","steps":[{"kind":"approve","description":"d"},]}"#;
        let repaired = repair_json(bad).expect("repair should succeed");
        let intent = NatlasEngineeringIntent::parse(&repaired).unwrap();
        assert_eq!(intent.steps.len(), 1);
    }

    #[test]
    fn repair_returns_none_when_no_defects() {
        let good = r#"{"summary":"s","steps":[{"kind":"approve","description":"d"}]}"#;
        assert!(repair_json(good).is_none(), "no repair needed for valid JSON");
    }

    #[test]
    fn parse_with_repair_detects_and_records_normalization() {
        // A triple-quoted string that strict parse rejects, but repair recovers.
        let bad = r#"{"summary":"s","steps":[{"kind":"implement","description":"d","path":"f.py","content":"""a
b"""}]}"#;
        let (intent, was_repaired) = NatlasEngineeringIntent::parse_with_repair(bad).unwrap();
        assert!(was_repaired, "must record that normalization occurred");
        assert_eq!(intent.steps[0].content.as_deref(), Some("a\nb"));
    }

    #[test]
    fn parse_with_repair_passes_through_valid_json_untouched() {
        let good = r#"{"summary":"s","steps":[{"kind":"approve","description":"d"}]}"#;
        let (intent, was_repaired) = NatlasEngineeringIntent::parse_with_repair(good).unwrap();
        assert!(!was_repaired, "valid JSON must not be marked as repaired");
        assert_eq!(intent.steps.len(), 1);
    }

    #[test]
    fn parse_with_repair_fails_closed_on_unrecoverable_defect() {
        // Missing closing brace — outside the bounded repair scope.
        let unrecoverable = r#"{"summary":"s","steps":["#;
        assert!(NatlasEngineeringIntent::parse_with_repair(unrecoverable).is_err());
    }
}
