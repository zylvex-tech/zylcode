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
//! command — is mapped to a side-effect-free [`TaskAction::Note`] in Phase C0,
//! because fabricating file contents or commands would be worse than not
//! executing them. The one exception is [`IntentStepKind::Approve`], which maps
//! to a **real** [`TaskAction::Approval`] gate: a human decision needs no
//! invented content, so it is executed for real.

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
    Approve,
}

impl IntentStepKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "analyse" | "analyze" => Some(Self::Analyse),
            "implement" => Some(Self::Implement),
            "test" => Some(Self::Test),
            "document" => Some(Self::Document),
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
            Self::Approve => "approve",
        }
    }
}

/// One step of a structured engineering intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatlasIntentStep {
    pub kind: IntentStepKind,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
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
            let kind_str = raw.get("kind").and_then(|v| v.as_str()).ok_or_else(|| {
                NatlasError::MalformedResponse {
                    detail: format!("step {i} is missing a string `kind`"),
                }
            })?;
            let kind = IntentStepKind::parse(kind_str).ok_or_else(|| {
                NatlasError::MalformedResponse {
                    detail: format!(
                        "step {i} has unknown kind `{kind_str}`; expected one of \
                         analyse|implement|test|document|approve"
                    ),
                }
            })?;
            let description = raw
                .get("description")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| NatlasError::MalformedResponse {
                    detail: format!("step {i} is missing a non-empty `description`"),
                })?
                .to_string();
            let target_path = raw
                .get("target_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            steps.push(NatlasIntentStep {
                kind,
                description,
                target_path,
            });
        }

        Ok(Self { summary, steps })
    }

    /// Convert the intent into a validated, dependency-chained factory task
    /// graph.
    ///
    /// The graph is what the existing `FactoryRunner` consumes, so this is the
    /// seam between the competition work and the product's durable workflow.
    pub fn to_task_graph(&self) -> Result<TaskGraph, NatlasError> {
        let mut graph = TaskGraph::new();
        let mut previous: Option<String> = None;

        for (i, step) in self.steps.iter().enumerate() {
            let id = format!("step-{:02}", i + 1);
            let (kind, action) = match step.kind {
                // Side-effect-free in C0: the description is recorded, nothing
                // is executed, because no real content or command is known yet.
                IntentStepKind::Analyse => (
                    TaskKind::RepositoryContext,
                    TaskAction::Note {
                        text: format!("analyse: {}", step.description),
                    },
                ),
                IntentStepKind::Implement => (
                    TaskKind::Implementation,
                    TaskAction::Note {
                        text: format!("implement (content not supplied): {}", step.description),
                    },
                ),
                IntentStepKind::Test => (
                    TaskKind::Test,
                    TaskAction::Note {
                        text: format!("test (command not supplied): {}", step.description),
                    },
                ),
                IntentStepKind::Document => (
                    TaskKind::Documentation,
                    TaskAction::Note {
                        text: format!("document: {}", step.description),
                    },
                ),
                // A real gate: no invented content is required, so this action
                // genuinely parks the job until a human approves.
                IntentStepKind::Approve => (
                    TaskKind::HumanApproval,
                    TaskAction::Approval {
                        reason: step.description.clone(),
                    },
                ),
            };

            let mut node = TaskNode::new(id.clone(), kind, step.description.clone(), action);
            if let Some(prev) = &previous {
                node = node.depends_on(&[prev.as_str()]);
            }
            graph
                .add_node(node)
                .map_err(|e| NatlasError::MalformedResponse {
                    detail: format!("task graph rejected step {id}: {e}"),
                })?;
            previous = Some(id);
        }

        graph.validate().map_err(|e| NatlasError::MalformedResponse {
            detail: format!("intent produced an invalid task graph: {e}"),
        })?;
        Ok(graph)
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
}
