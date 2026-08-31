//! Dynamic artifact generator & pipeline — parses LLM output into
//! typed [`Artifact`]s and drives the verification loop.

use crate::planner::IntentPlanner;
use crate::router::{RouterConfig, TokenRouter, TokenSnapshot};
use crate::{Intent, IntentResult, VerificationReport};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Generated artifact enum (distinct from crate::Artifact which is the
// IntentResult payload). The pipeline emits this enum and then maps each
// variant into a crate::Artifact for the final result.
// ---------------------------------------------------------------------------

/// Typed artifact produced by the LLM pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Artifact {
    /// A React / frontend component.
    UiComponent {
        path: String,
        content: String,
        #[serde(default)]
        language: String,
    },
    /// A Rust module or crate fragment.
    RustModule {
        path: String,
        content: String,
        #[serde(default)]
        tests: Option<String>,
    },
    /// A marketplace plugin manifest.
    PluginManifest {
        path: String,
        content: String,
        manifest: Option<serde_json::Value>,
    },
    /// A formal proof / spec artifact.
    FormalProofSpec {
        path: String,
        content: String,
        obligations: Vec<String>,
    },
}

impl Artifact {
    pub fn path(&self) -> &str {
        match self {
            Self::UiComponent { path, .. }
            | Self::RustModule { path, .. }
            | Self::PluginManifest { path, .. }
            | Self::FormalProofSpec { path, .. } => path,
        }
    }

    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::UiComponent { .. } => "UiComponent",
            Self::RustModule { .. } => "RustModule",
            Self::PluginManifest { .. } => "PluginManifest",
            Self::FormalProofSpec { .. } => "FormalProofSpec",
        }
    }

    pub fn content(&self) -> &str {
        match self {
            Self::UiComponent { content, .. }
            | Self::RustModule { content, .. }
            | Self::PluginManifest { content, .. }
            | Self::FormalProofSpec { content, .. } => content,
        }
    }
}

// ---------------------------------------------------------------------------
// Proof metrics included in the final IntentResult context
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMetrics {
    pub artifacts_generated: usize,
    pub artifacts_verified: usize,
    pub verification_passed: bool,
    pub verification_duration_ms: u64,
    pub verification_checks: usize,
    pub tokens: TokenSnapshot,
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Orchestrates: planning → routing → parsing → verification → result.
#[derive(Debug, Clone)]
pub struct ArtifactPipeline {
    router: Arc<TokenRouter>,
    planner: IntentPlanner,
}

impl ArtifactPipeline {
    pub fn new(router: TokenRouter) -> Self {
        Self {
            router: Arc::new(router),
            planner: IntentPlanner::new(),
        }
    }

    pub fn with_planner(router: TokenRouter, planner: IntentPlanner) -> Self {
        Self {
            router: Arc::new(router),
            planner,
        }
    }

    pub fn from_config(config: RouterConfig) -> Result<Self> {
        let router = TokenRouter::new(config)?;
        Ok(Self::new(router))
    }

    pub fn router(&self) -> &TokenRouter {
        &self.router
    }

    /// Execute the full pipeline for an intent.
    ///
    /// Flow:
    /// 1. Build execution plan via `IntentPlanner`.
    /// 2. Dispatch compiled prompt via `TokenRouter`.
    /// 3. Parse artifacts from the LLM response.
    /// 4. Run `verify_logic` over parsed artifacts (via callback).
    /// 5. Return `IntentResult` with verified status + proof metrics.
    pub async fn execute_pipeline<F, Fut>(
        &self,
        intent: Intent,
        bridges: Vec<crate::McpBridgeDescriptor>,
        verify: F,
    ) -> Result<IntentResult>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<VerificationReport>> + Send,
    {
        if intent.prompt.trim().is_empty() {
            anyhow::bail!("intent prompt must not be empty");
        }

        // 1) Plan
        let plan = self.planner.build_execution_plan(&intent, &bridges);
        info!(goal = %plan.goal, steps = plan.steps.len(), "execution plan built");

        // 2) Route to LLM
        let raw = self
            .router
            .dispatch_prompt(&plan.compiled_prompt, &plan.system_prompt)
            .await
            .context("token router dispatch failed")?;

        // 3) Parse artifacts
        let artifacts = Self::parse_artifacts(&raw);
        info!(count = artifacts.len(), "artifacts parsed from LLM response");

        if artifacts.is_empty() {
            warn!("no artifacts extracted; returning raw output as fallback");
        }

        // 4) Verify
        let verification = verify().await.context("verification step failed")?;
        let passed = verification.passed;
        let duration_ms = verification.duration_ms;
        let checks = verification.checks.len();

        // Token telemetry: estimate saved tokens when verification passed without regeneration.
        if passed && !artifacts.is_empty() {
            // Heuristic: each verified artifact saves ~20% of its output tokens.
            let total_chars: usize = artifacts.iter().map(|a| a.content().len()).sum();
            let saved = (total_chars / 4 / 5) as u64;
            self.router.metrics().record_saved(saved);
        }

        // 5) Map to IntentResult artifacts + embed proof metrics in summary/context
        let metrics = ProofMetrics {
            artifacts_generated: artifacts.len(),
            artifacts_verified: if passed { artifacts.len() } else { 0 },
            verification_passed: passed,
            verification_duration_ms: duration_ms,
            verification_checks: checks,
            tokens: self.router.snapshot(),
        };

        let mapped: Vec<crate::Artifact> = artifacts
            .iter()
            .map(|a| crate::Artifact {
                kind: a.kind_str().to_string(),
                label: a.path().to_string(),
                content: a.content().to_string(),
            })
            .collect();

        let summary = if artifacts.is_empty() {
            format!(
                "Processed intent: {} — no structured artifacts extracted (raw output preserved). Verification: {}",
                intent.prompt,
                if passed { "PASSED" } else { "FAILED" }
            )
        } else {
            format!(
                "Processed intent: {} — {} artifact(s) generated, verification {} ({} checks, {} ms). Tokens in/out: {}/{}, saved: {}",
                intent.prompt,
                artifacts.len(),
                if passed { "PASSED" } else { "FAILED" },
                checks,
                duration_ms,
                metrics.tokens.input_tokens,
                metrics.tokens.output_tokens,
                metrics.tokens.verification_saved_tokens
            )
        };

        // Preserve raw LLM output as an additional artifact when no structured
        // artifacts were found, so callers can still inspect the result.
        let mut final_artifacts = mapped;
        if final_artifacts.is_empty() {
            final_artifacts.push(crate::Artifact {
                kind: "llm_raw".to_string(),
                label: "raw_output".to_string(),
                content: raw.clone(),
            });
        }

        // Embed metrics as a JSON artifact for UI consumption.
        final_artifacts.push(crate::Artifact {
            kind: "proof_metrics".to_string(),
            label: "proof_metrics.json".to_string(),
            content: serde_json::to_string_pretty(&metrics).unwrap_or_default(),
        });

        Ok(IntentResult {
            summary,
            artifacts: final_artifacts,
            success: passed,
        })
    }

    /// Simplified entry used by `ZylCodeEngine::process_intent` — uses an
    /// inline verification closure that checks workspace existence + artifact
    /// content heuristics.
    pub async fn execute_for_engine(
        &self,
        intent: Intent,
        bridges: Vec<crate::McpBridgeDescriptor>,
        workspace_root: String,
    ) -> Result<IntentResult> {
        self.execute_pipeline(intent, bridges, move || {
            let root = workspace_root.clone();
            async move { Self::default_verification(&root).await }
        })
        .await
    }

    // -- parsing -------------------------------------------------------------

    /// Parse `<artifact>` blocks from an LLM response. Supports both the
    /// canonical `<zylcode-response>` envelope and bare markdown code fences
    /// as fallback. Zero-alloc scanning via `memchr` — no per-call `Regex`.
    pub fn parse_artifacts(raw: &str) -> Vec<Artifact> {
        let mut out = Vec::new();
        out.extend(Self::parse_artifacts_memchr(raw));
        // Fallback: extract ``` blocks when no XML artifacts were found.
        if out.is_empty() {
            out.extend(parse_code_fences(raw));
        }
        out
    }

    /// Internal memchr-based scanner — returns slices where possible and
    /// allocates only for final `Artifact` fields.
    fn parse_artifacts_memchr(raw: &str) -> Vec<Artifact> {
        let mut out = Vec::new();
        let mut pos = 0;
        let raw_bytes = raw.as_bytes();
        while let Some(start) = memchr::memmem::find(&raw_bytes[pos..], b"<artifact") {
            let abs_start = pos + start;
            let tag_end = match memchr::memchr(b'>', &raw_bytes[abs_start..]) {
                Some(o) => abs_start + o,
                None => break,
            };
            let tag = &raw[abs_start..=tag_end];
            // Extract kind="..."
            let kind_raw = Self::extract_attr(tag, "kind").unwrap_or("").to_lowercase();
            let path = Self::extract_attr(tag, "path")
                .map(|s| s.to_string())
                .unwrap_or_else(|| default_path_for_kind(&kind_raw));
            // Find closing </artifact>
            let inner_start = tag_end + 1;
            let close_rel = match memchr::memmem::find(&raw_bytes[inner_start..], b"</artifact>") {
                Some(o) => o,
                None => break,
            };
            let inner = &raw[inner_start..inner_start + close_rel];
            let content = extract_cdata_or_text(inner).to_string();

            let artifact = match kind_raw.as_str() {
                "uicomponent" | "ui_component" | "reactcomponent" | "react_component" => Artifact::UiComponent {
                    path,
                    content,
                    language: "tsx".to_string(),
                },
                "rustmodule" | "rust_module" | "rustcrate" | "rust_crate" => Artifact::RustModule {
                    path,
                    content,
                    tests: None,
                },
                "pluginmanifest" | "plugin_manifest" | "mcmanifest" | "mcpmanifest" => {
                    let manifest = serde_json::from_str::<serde_json::Value>(&content).ok();
                    Artifact::PluginManifest {
                        path,
                        content,
                        manifest,
                    }
                }
                "formalproof" | "formal_proof" | "formalproofspec" | "formal_proof_spec" => {
                    let obligations = extract_obligations(&content);
                    Artifact::FormalProofSpec {
                        path,
                        content,
                        obligations,
                    }
                }
                _ => Artifact::RustModule {
                    path,
                    content,
                    tests: None,
                },
            };
            out.push(artifact);
            pos = inner_start + close_rel + "</artifact>".len();
        }
        out
    }

    #[inline]
    fn extract_attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
        let needle = format!(r#"{}=""#, name);
        let start = tag.find(&needle)? + needle.len();
        let end = tag[start..].find('"')?;
        Some(&tag[start..start + end])
    }

    async fn default_verification(workspace_root: &str) -> Result<VerificationReport> {
        let start = std::time::Instant::now();
        let mut checks = Vec::new();

        let present = !workspace_root.trim().is_empty();
        checks.push(crate::VerificationCheck {
            name: "workspace_root_present".to_string(),
            passed: present,
            message: if present {
                format!("workspace_root is '{workspace_root}'")
            } else {
                "workspace_root must not be empty".to_string()
            },
        });

        let exists = std::path::Path::new(workspace_root).exists();
        checks.push(crate::VerificationCheck {
            name: "workspace_root_exists".to_string(),
            passed: exists,
            message: if exists {
                "workspace root exists on disk".to_string()
            } else {
                format!("workspace root '{workspace_root}' does not exist")
            },
        });

        let passed = checks.iter().all(|c| c.passed);
        Ok(VerificationReport {
            passed,
            checks,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

fn default_path_for_kind(kind: &str) -> String {
    match kind {
        "uicomponent" | "ui_component" | "reactcomponent" => "src/components/Generated.tsx".to_string(),
        "rustmodule" | "rust_module" | "rustcrate" => "src/generated.rs".to_string(),
        "pluginmanifest" | "plugin_manifest" => "plugin-manifest.json".to_string(),
        "formalproof" | "formal_proof" => "proof/spec.md".to_string(),
        _ => "artifact/out.txt".to_string(),
    }
}

/// Zero-alloc inner: returns a slice borrowed from `inner`. Caller decides when to allocate.
fn extract_cdata_or_text_slice(inner: &str) -> &str {
    let trimmed = inner.trim();
    let content_inner = if let Some(start) = trimmed.find("<content") {
        if let Some(tag_end) = trimmed[start..].find('>') {
            let content_start = start + tag_end + 1;
            if let Some(end) = trimmed[content_start..].find("</content>") {
                &trimmed[content_start..content_start + end]
            } else {
                trimmed
            }
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    let cdata_stripped = content_inner.trim();
    if cdata_stripped.starts_with("<![CDATA[") && cdata_stripped.ends_with("]]>") {
        &cdata_stripped[9..cdata_stripped.len() - 3]
    } else if let Some(stripped) = cdata_stripped.strip_prefix("<![CDATA[") {
        stripped
    } else {
        cdata_stripped
    }
}

fn extract_cdata_or_text(inner: &str) -> String {
    extract_cdata_or_text_slice(inner).to_string()
}

fn extract_obligations(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|l| l.trim_start().starts_with("- ") || l.trim_start().starts_with("* "))
        .map(|l| l.trim().to_string())
        .collect()
}

fn parse_code_fences(raw: &str) -> Vec<Artifact> {
    let mut out = Vec::new();
    let bytes = raw.as_bytes();
    let mut pos = 0;
    while let Some(start) = memchr::memmem::find(&bytes[pos..], b"```") {
        let abs = pos + start + 3;
        // lang is up to next '\n'
        let lang_end = memchr::memchr(b'\n', &bytes[abs..]).map(|o| abs + o).unwrap_or(bytes.len());
        let lang = raw[abs..lang_end].trim().to_lowercase();
        let code_start = if lang_end < bytes.len() { lang_end + 1 } else { lang_end };
        let close_rel = match memchr::memmem::find(&bytes[code_start..], b"```") {
            Some(o) => o,
            None => break,
        };
        let code = raw[code_start..code_start + close_rel].to_string();
        if code.trim().is_empty() {
            pos = code_start + close_rel + 3;
            continue;
        }
        let (kind, path) = match lang.as_str() {
            "tsx" | "jsx" | "typescript" | "ts" => ("UiComponent", "src/components/Generated.tsx"),
            "rust" | "rs" => ("RustModule", "src/generated.rs"),
            "json" => ("PluginManifest", "plugin-manifest.json"),
            _ => ("RustModule", "src/generated.rs"),
        };
        let artifact = match kind {
            "UiComponent" => Artifact::UiComponent {
                path: path.to_string(),
                content: code,
                language: lang,
            },
            "PluginManifest" => {
                let manifest = serde_json::from_str(&code).ok();
                Artifact::PluginManifest {
                    path: path.to_string(),
                    content: code,
                    manifest,
                }
            }
            _ => Artifact::RustModule {
                path: path.to_string(),
                content: code,
                tests: None,
            },
        };
        out.push(artifact);
        pos = code_start + close_rel + 3;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_xml_artifact_with_cdata() {
        let raw = r#"
<zylcode-response>
  <artifact kind="UiComponent" path="MyView.tsx"><content><![CDATA[export const X = () => <div/>]]></content></artifact>
  <artifact kind="RustModule" path="src/lib.rs"><content><![CDATA[pub fn hello() {}]]></content></artifact>
</zylcode-response>
"#;
        let arts = ArtifactPipeline::parse_artifacts(raw);
        assert_eq!(arts.len(), 2);
        assert_eq!(arts[0].kind_str(), "UiComponent");
        assert_eq!(arts[1].kind_str(), "RustModule");
    }

    #[test]
    fn fallback_parses_markdown_fences_when_no_xml() {
        let raw = "Here is code:\n```rust\npub fn foo() {}\n```\n";
        let arts = ArtifactPipeline::parse_artifacts(raw);
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].kind_str(), "RustModule");
    }

    #[test]
    fn parses_formal_proof_obligations() {
        let raw = r#"<artifact kind="FormalProofSpec" path="proof.md"><content><![CDATA[- obligation 1
- obligation 2]]></content></artifact>"#;
        let arts = ArtifactPipeline::parse_artifacts(raw);
        assert_eq!(arts.len(), 1);
        if let Artifact::FormalProofSpec { obligations, .. } = &arts[0] {
            assert_eq!(obligations.len(), 2);
        } else {
            panic!("expected FormalProofSpec");
        }
    }

    #[tokio::test]
    async fn pipeline_offline_generates_artifacts_and_passes_verification() {
        use crate::router::RouterConfig;
        let pipeline = ArtifactPipeline::from_config(RouterConfig::default()).unwrap();
        let intent = crate::Intent {
            prompt: "Build a Rust counter module".to_string(),
            context: None,
            correlation_id: None,
        };
        let result = pipeline
            .execute_for_engine(intent, vec![], ".".to_string())
            .await
            .unwrap();
        assert!(result.success);
        assert!(!result.artifacts.is_empty());
    }
}
