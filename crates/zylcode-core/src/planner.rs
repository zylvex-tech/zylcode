//! LLM planner & context assembler — turns an [`Intent`] + MCP bridge
//! schemas into a structured [`ExecutionPlan`] and the final prompt strings
//! sent to the token router.

use crate::{Intent, McpBridgeDescriptor};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Execution plan types
// ---------------------------------------------------------------------------

/// A single resolution step within an execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Ordered index (1-based for display).
    pub index: u32,
    /// Short title (e.g. `Generate React component`).
    pub title: String,
    /// Detailed instruction for the LLM.
    pub instruction: String,
    /// Tools / capabilities required (e.g. `fs:write`, `mcp:fetch`).
    #[serde(default)]
    pub required_tools: Vec<String>,
    /// Whether the step requires formal verification.
    #[serde(default)]
    pub requires_verification: bool,
}

/// A complete execution plan produced by [`IntentPlanner`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Human-readable goal derived from the intent.
    pub goal: String,
    /// Ordered steps to achieve the goal.
    pub steps: Vec<PlanStep>,
    /// Capabilities contributed by registered MCP bridges.
    #[serde(default)]
    pub mcp_capabilities: Vec<String>,
    /// System preamble to prepend to the LLM prompt.
    pub system_prompt: String,
    /// Assembled user prompt (intent + context + tool manifests).
    pub compiled_prompt: String,
}

// ---------------------------------------------------------------------------
// Planner
// ---------------------------------------------------------------------------

/// Assembles intents and MCP bridge schemas into an [`ExecutionPlan`].
#[derive(Debug, Clone, Default)]
pub struct IntentPlanner {
    /// Extra system instruction prefix (injected before the default).
    pub system_prefix: Option<String>,
}

impl IntentPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_system_prefix(prefix: impl Into<String>) -> Self {
        Self {
            system_prefix: Some(prefix.into()),
        }
    }

    /// Build a full execution plan from an intent and the active bridges.
    pub fn build_execution_plan(
        &self,
        intent: &Intent,
        bridges: &[McpBridgeDescriptor],
    ) -> ExecutionPlan {
        let goal = intent.prompt.trim().to_string();

        let mcp_capabilities = bridges
            .iter()
            .map(|b| format!("{} ({}) — {}", b.id, b.transport, b.endpoint))
            .collect::<Vec<_>>();

        let system_prompt = self.build_system_prompt(bridges);
        let compiled_prompt = self.build_compiled_prompt(intent, bridges);

        let steps = self.derive_steps(intent, bridges);

        ExecutionPlan {
            goal,
            steps,
            mcp_capabilities,
            system_prompt,
            compiled_prompt,
        }
    }

    /// Convenience: build a plan with no MCP bridges (e.g. for tests).
    pub fn build_plan_for_intent(&self, intent: &Intent) -> ExecutionPlan {
        self.build_execution_plan(intent, &[])
    }

    // -- internal builders --------------------------------------------------

    fn build_system_prompt(&self, bridges: &[McpBridgeDescriptor]) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(prefix) = &self.system_prefix {
            parts.push(prefix.clone());
        }

        parts.push(
            "You are ZylCode — an autonomous developer engine. \
             You produce structured artifacts wrapped in <zylcode-response> XML. \
             Each <artifact> must declare kind (UiComponent | RustModule | PluginManifest | FormalProofSpec), \
             a file path, and CDATA content. \
             Produce at least one artifact. Keep code production-ready and compilable."
                .to_string(),
        );

        if !bridges.is_empty() {
            parts.push(format!(
                "Available MCP bridges ({}):\n{}",
                bridges.len(),
                bridges
                    .iter()
                    .map(|b| format!("- {} [{}] {}", b.id, b.transport, b.endpoint))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
            parts.push(
                "When relevant, emit tool-call specs as <tool-call bridge=\"id\" method=\"...\"> JSON payloads inside the response.".to_string(),
            );
        }

        parts.push(
            "Response format:\n\
             <zylcode-response>\n  \
               <artifact kind=\"...\" path=\"...\">CDATA</artifact>\n  \
               ...\n\
             </zylcode-response>"
                .to_string(),
        );

        parts.join("\n\n")
    }

    fn build_compiled_prompt(&self, intent: &Intent, bridges: &[McpBridgeDescriptor]) -> String {
        let mut out = String::new();
        out.push_str("## Intent\n");
        out.push_str(&intent.prompt);
        out.push('\n');

        if let Some(ctx) = &intent.context {
            out.push_str("\n## Context\n");
            out.push_str(&serde_json::to_string_pretty(ctx).unwrap_or_else(|_| ctx.to_string()));
            out.push('\n');
        }

        if let Some(corr) = &intent.correlation_id {
            out.push_str("\n## Correlation ID\n");
            out.push_str(corr);
            out.push('\n');
        }

        if !bridges.is_empty() {
            out.push_str("\n## Registered MCP Bridges\n");
            for b in bridges {
                out.push_str(&format!(
                    "- id: {} | transport: {} | endpoint: {} | env_keys: {}\n",
                    b.id,
                    b.transport,
                    b.endpoint,
                    b.env.keys().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
        }

        out.push_str(
            "\n## Instructions\n\
             Decompose the intent into artifacts. \
             For code artifacts, ensure they are self-contained and verifiable. \
             Wrap every artifact in the XML envelope described in the system prompt.\n",
        );

        out
    }

    fn derive_steps(&self, intent: &Intent, bridges: &[McpBridgeDescriptor]) -> Vec<PlanStep> {
        let prompt_lc = intent.prompt.to_lowercase();
        let mut steps: Vec<PlanStep> = Vec::new();
        let mut idx: u32 = 1;

        // Heuristic step derivation — deterministic, no LLM call.
        // The LLM itself will produce the actual artifacts; these steps are for
        // UI progress reporting and verification orchestration.

        let needs_ui = prompt_lc.contains("component")
            || prompt_lc.contains("ui")
            || prompt_lc.contains("react")
            || prompt_lc.contains("view")
            || prompt_lc.contains("page");

        let needs_rust = prompt_lc.contains("crate")
            || prompt_lc.contains("rust")
            || prompt_lc.contains("module")
            || prompt_lc.contains("function");

        let needs_plugin = prompt_lc.contains("plugin") || prompt_lc.contains("extension") || prompt_lc.contains("manifest");

        if needs_ui {
            steps.push(PlanStep {
                index: idx,
                title: "Generate UI component".to_string(),
                instruction: "Produce a React component artifact (UiComponent) that satisfies the intent.".to_string(),
                required_tools: vec!["artifact:react".to_string()],
                requires_verification: true,
            });
            idx += 1;
        }

        if needs_rust {
            steps.push(PlanStep {
                index: idx,
                title: "Generate Rust module".to_string(),
                instruction: "Produce a Rust module artifact (RustModule) with public API and unit tests.".to_string(),
                required_tools: vec!["artifact:rust".to_string()],
                requires_verification: true,
            });
            idx += 1;
        }

        if needs_plugin {
            steps.push(PlanStep {
                index: idx,
                title: "Generate plugin manifest".to_string(),
                instruction: "Produce a PluginManifest artifact containing id, version, and capabilities.".to_string(),
                required_tools: vec!["artifact:manifest".to_string()],
                requires_verification: true,
            });
            idx += 1;
        }

        // Fallback generic step when heuristics match nothing.
        if steps.is_empty() {
            steps.push(PlanStep {
                index: 1,
                title: "Generate solution artifacts".to_string(),
                instruction: format!("Produce artifacts that fulfill: {}", intent.prompt),
                required_tools: vec!["artifact:generic".to_string()],
                requires_verification: true,
            });
        }

        // If MCP bridges are registered, add a bridge-aware step.
        if !bridges.is_empty() {
            steps.push(PlanStep {
                index: idx,
                title: "Wire MCP bridge capabilities".to_string(),
                instruction: format!(
                    "Incorporate available MCP bridges ({}) into the solution where applicable.",
                    bridges.iter().map(|b| b.id.as_str()).collect::<Vec<_>>().join(", ")
                ),
                required_tools: bridges.iter().map(|b| format!("mcp:{}", b.id)).collect(),
                requires_verification: false,
            });
        }

        // Always end with verification.
        steps.push(PlanStep {
            index: steps.len() as u32 + 1,
            title: "Formal verification".to_string(),
            instruction: "Run verify_logic over all generated artifacts and surface proof metrics.".to_string(),
            required_tools: vec!["verify:logic".to_string()],
            requires_verification: false,
        });

        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Intent;

    fn intent(prompt: &str) -> Intent {
        Intent {
            prompt: prompt.to_string(),
            context: None,
            correlation_id: None,
        }
    }

    #[test]
    fn plan_contains_ui_step_when_prompt_mentions_component() {
        let planner = IntentPlanner::new();
        let plan = planner.build_plan_for_intent(&intent("Build a React component for a counter"));
        assert!(plan.steps.iter().any(|s| s.title.contains("UI component")));
    }

    #[test]
    fn plan_includes_mcp_capabilities_when_bridges_present() {
        let planner = IntentPlanner::new();
        let bridges = vec![McpBridgeDescriptor {
            id: "fs".to_string(),
            endpoint: "npx fs-mcp".to_string(),
            transport: "stdio".to_string(),
            env: Default::default(),
        }];
        let plan = planner.build_execution_plan(&intent("do something"), &bridges);
        assert_eq!(plan.mcp_capabilities.len(), 1);
        assert!(plan.system_prompt.contains("MCP bridges"));
    }

    #[test]
    fn system_prompt_contains_xml_envelope_instruction() {
        let planner = IntentPlanner::new();
        let plan = planner.build_plan_for_intent(&intent("hello"));
        assert!(plan.system_prompt.contains("<zylcode-response>"));
    }
}
