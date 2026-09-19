//! Canonical tool catalogue — the single source of truth for what exists and
//! what can actually run.
//!
//! # Architectural invariant
//!
//! ```text
//!     catalogue definition  !=  executable capability
//! ```
//!
//! A [`ToolDefinition`](crate::enhanced_bridge::ToolDefinition) is a *claim*. An
//! executable capability requires all of:
//!
//! ```text
//!     definition
//!   + valid parameter schema
//!   + real executor
//!   + permission / risk policy
//!   + deterministic dispatch binding      (the tool id constrains the operation)
//!   + reachable named product path        (required for R3)
//!   + captured execution evidence         (required for R3)
//! ```
//!
//! # Why there is no single "tool count"
//!
//! Quantity is not a correctness invariant. A catalogue of 150 names attached to
//! a simulator is worth less than twelve tools that actually execute. This module
//! therefore exposes five explicit, separately-scoped metrics and deliberately
//! offers no aggregate "tool count" for anything to assert on:
//!
//! | metric | counts |
//! |---|---|
//! | [`CatalogueMetrics::definition_count`] | every catalogued definition |
//! | [`CatalogueMetrics::executable_count`] | definitions with a real executor |
//! | [`CatalogueMetrics::tested_execution_count`] | executable **and** exercised by a committed test |
//! | [`CatalogueMetrics::product_reachable_count`] | executable **and** reachable from a product surface |
//! | [`CatalogueMetrics::r3_verified_count`] | exercised through a product surface with captured evidence |
//!
//! # Provenance
//!
//! Derived from `docs/governance/TOOL_CATALOGUE_TRUTH_TABLE.md`. The bridge is
//! 0 REAL / 27 SIMULATED / 27 UNREACHABLE; `real_tools` provides 12 real
//! executors. The historical `>= 100` / `>= 150` / `>= 25` assertions were
//! calibrated to a tool surface that was never committed and are not product
//! requirements.

use crate::real_tools::{get_real_tool, RiskLevel, ToolPermissions};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Whether a catalogued entry can actually be executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    /// Definition and schema exist; no real executor does.
    ///
    /// These MUST NOT be registered in the executable registry and MUST NOT be
    /// reported as successes. They are preserved metadata, nothing more.
    DefinitionOnly,
    /// Definition + schema + real executor + deterministic dispatch binding.
    Executable,
}

impl CapabilityStatus {
    pub fn is_executable(self) -> bool {
        matches!(self, Self::Executable)
    }
}

/// Evidence rung, per `docs/governance/ZYLCODE_PROOF_GRAPH.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRung {
    /// Claimed, not observed.
    R0Claimed,
    /// Observed in source.
    R1Observed,
    /// Executed by a committed test.
    R2Executed,
    /// Reachable through a named product surface with captured evidence.
    R3Verified,
    /// Reproducible from a documented procedure.
    R4Reproducible,
    /// Commissioned in production.
    R5Commissioned,
}

impl EvidenceRung {
    pub fn label(self) -> &'static str {
        match self {
            Self::R0Claimed => "R0",
            Self::R1Observed => "R1",
            Self::R2Executed => "R2",
            Self::R3Verified => "R3",
            Self::R4Reproducible => "R4",
            Self::R5Commissioned => "R5",
        }
    }
}

/// A single catalogue entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEntry {
    pub id: String,
    pub description: String,
    /// Parameter schema. Present for every entry, including definition-only
    /// ones — the schema is the asset worth preserving.
    pub input_schema: Value,
    pub risk: RiskLevel,
    pub permissions: ToolPermissions,
    pub status: CapabilityStatus,
    pub rung: EvidenceRung,
    /// Whether a named product surface can reach this tool.
    pub product_reachable: bool,
    /// Human-readable statement of the operation this tool id is bound to.
    /// `None` for definition-only entries, which have no executor at all.
    pub bound_operation: Option<String>,
}

/// Explicit, non-ambiguous counts. See the module docs for why there is no
/// aggregate "tool count".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogueMetrics {
    pub definition_count: usize,
    pub executable_count: usize,
    pub tested_execution_count: usize,
    pub product_reachable_count: usize,
    pub r3_verified_count: usize,
}

/// The canonical catalogue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalogue {
    entries: Vec<ToolEntry>,
}

/// Static description of one real executor.
struct ExecutableSpec {
    id: &'static str,
    description: &'static str,
    schema: fn() -> Value,
    risk: RiskLevel,
    /// The operation the executor is bound to. `None` means unbound.
    bound_operation: Option<&'static str>,
    /// Exercised by a committed test (R2).
    tested: bool,
    /// Reachable from a named product surface.
    product_reachable: bool,
    /// Evidence rung currently attained.
    rung: EvidenceRung,
}

fn read_path_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Path to read, relative to the working directory"}
        },
        "required": ["path"]
    })
}

fn write_path_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Path to write, relative to the working directory"},
            "content": {"type": "string", "description": "Full file content to write"}
        },
        "required": ["path", "content"]
    })
}

fn list_path_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Directory to list; defaults to the working directory"}
        }
    })
}

fn generic_shell_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "command": {"type": "string", "description": "Program to execute"},
            "args": {"type": "array", "items": {"type": "string"}, "description": "Arguments"}
        },
        "required": ["command"]
    })
}

fn bound_args_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "args": {"type": "array", "items": {"type": "string"}, "description": "Trailing arguments; the program is fixed by the tool id"}
        }
    })
}

fn echo_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "message": {"type": "string", "description": "Text to echo"}
        },
        "required": ["message"]
    })
}

fn git_commit_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "message": {"type": "string", "description": "Commit message"},
            "files": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Paths to stage before committing; empty stages nothing extra"
            }
        },
        "required": ["message"]
    })
}

fn search_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "pattern": {"type": "string", "description": "Pattern to search for"},
            "path": {"type": "string", "description": "Root to search under; defaults to the working directory"}
        },
        "required": ["pattern"]
    })
}

/// The executable foundation. Every id here has a real executor in
/// [`crate::real_tools`] **and** a deterministic dispatch binding enforced by
/// that executor.
///
/// Note the evidence rungs. Only `fs.read` and `shell.execute` are R2 — the
/// rest are R1 (observed in source, not yet exercised by a committed test).
/// Nothing here is R3: no capability has been exercised through a named product
/// surface with captured evidence.
const EXECUTABLE_SPECS: &[ExecutableSpec] = &[
    ExecutableSpec {
        id: "fs.read",
        description: "Read a file's contents",
        schema: read_path_schema,
        risk: RiskLevel::Read,
        bound_operation: Some("fs.read"),
        tested: true,
        product_reachable: true,
        rung: EvidenceRung::R2Executed,
    },
    ExecutableSpec {
        id: "fs.write",
        description: "Write a file's contents",
        schema: write_path_schema,
        risk: RiskLevel::Write,
        bound_operation: Some("fs.write"),
        tested: false,
        product_reachable: true,
        rung: EvidenceRung::R1Observed,
    },
    ExecutableSpec {
        id: "fs.list",
        description: "List a directory",
        schema: list_path_schema,
        risk: RiskLevel::Read,
        bound_operation: Some("fs.list"),
        tested: false,
        product_reachable: true,
        rung: EvidenceRung::R1Observed,
    },
    ExecutableSpec {
        id: "shell.execute",
        description: "Execute an arbitrary program. The escape hatch: no program binding, highest risk.",
        schema: generic_shell_schema,
        risk: RiskLevel::Execute,
        bound_operation: None,
        tested: true,
        product_reachable: true,
        rung: EvidenceRung::R2Executed,
    },
    ExecutableSpec {
        id: "shell.echo",
        description: "Echo text. Bound to the `echo` program only.",
        schema: echo_schema,
        risk: RiskLevel::Execute,
        bound_operation: Some("echo"),
        tested: false,
        product_reachable: true,
        rung: EvidenceRung::R1Observed,
    },
    ExecutableSpec {
        id: "npm.run",
        description: "Run an npm script. Bound to `npm run` only.",
        schema: bound_args_schema,
        risk: RiskLevel::Execute,
        bound_operation: Some("npm run"),
        tested: false,
        product_reachable: true,
        rung: EvidenceRung::R1Observed,
    },
    ExecutableSpec {
        id: "cargo.test",
        description: "Run the Cargo test suite. Bound to `cargo test` only.",
        schema: bound_args_schema,
        risk: RiskLevel::Execute,
        bound_operation: Some("cargo test"),
        tested: false,
        product_reachable: true,
        rung: EvidenceRung::R1Observed,
    },
    ExecutableSpec {
        id: "git.status",
        description: "Show working-tree status. Bound to `git status` only.",
        schema: bound_args_schema,
        risk: RiskLevel::Read,
        bound_operation: Some("git status"),
        tested: false,
        product_reachable: true,
        rung: EvidenceRung::R1Observed,
    },
    ExecutableSpec {
        id: "git.diff",
        description: "Show changes. Bound to `git diff` only.",
        schema: bound_args_schema,
        risk: RiskLevel::Read,
        bound_operation: Some("git diff"),
        tested: false,
        product_reachable: true,
        rung: EvidenceRung::R1Observed,
    },
    ExecutableSpec {
        id: "git.commit",
        description: "Create a commit. Bound to `git commit` only; cannot push, reset, clean, checkout, rebase or reconfigure.",
        schema: git_commit_schema,
        risk: RiskLevel::GitWrite,
        bound_operation: Some("git commit"),
        tested: false,
        product_reachable: false,
        rung: EvidenceRung::R1Observed,
    },
    ExecutableSpec {
        id: "search.find",
        description: "Find files by name pattern",
        schema: search_schema,
        risk: RiskLevel::Read,
        bound_operation: Some("search mode `filename`"),
        tested: false,
        product_reachable: true,
        rung: EvidenceRung::R1Observed,
    },
    ExecutableSpec {
        id: "search.grep",
        description: "Search file contents by pattern",
        schema: search_schema,
        risk: RiskLevel::Read,
        bound_operation: Some("search mode `text`"),
        tested: false,
        product_reachable: true,
        rung: EvidenceRung::R1Observed,
    },
];

fn default_permissions_for(risk: RiskLevel) -> ToolPermissions {
    match risk {
        RiskLevel::Read => ToolPermissions {
            read: true,
            write: false,
            execute: false,
            network: false,
            destructive: false,
        },
        RiskLevel::Write => ToolPermissions {
            read: true,
            write: true,
            execute: false,
            network: false,
            destructive: false,
        },
        RiskLevel::Execute => ToolPermissions {
            read: true,
            write: false,
            execute: true,
            network: false,
            destructive: false,
        },
        RiskLevel::Network => ToolPermissions {
            read: true,
            write: false,
            execute: false,
            network: true,
            destructive: false,
        },
        RiskLevel::GitWrite => ToolPermissions {
            read: true,
            write: true,
            execute: true,
            network: false,
            destructive: false,
        },
        RiskLevel::Destructive | RiskLevel::Release => ToolPermissions {
            read: true,
            write: true,
            execute: true,
            network: false,
            destructive: true,
        },
    }
}

impl Catalogue {
    /// Build the canonical catalogue.
    ///
    /// Executable entries come from [`EXECUTABLE_SPECS`]. Every remaining
    /// definition found in the enhanced bridge is preserved as
    /// [`CapabilityStatus::DefinitionOnly`] metadata — its schema is kept, and
    /// it is explicitly *not* executable.
    ///
    /// An id that has a real executor is classified `Executable` even if the
    /// bridge also defines it. That overlap is exactly the conflation this
    /// module exists to remove: `git.commit` appears in both systems, and only
    /// one of them can run.
    pub fn canonical() -> Self {
        let mut entries: Vec<ToolEntry> = EXECUTABLE_SPECS
            .iter()
            .map(|spec| {
                let executable = get_real_tool(spec.id).is_some();
                debug_assert!(
                    executable,
                    "EXECUTABLE_SPECS lists `{}` but get_real_tool() returns None for it",
                    spec.id
                );
                ToolEntry {
                    id: spec.id.to_string(),
                    description: spec.description.to_string(),
                    input_schema: (spec.schema)(),
                    risk: spec.risk,
                    permissions: default_permissions_for(spec.risk),
                    status: CapabilityStatus::Executable,
                    rung: spec.rung,
                    product_reachable: spec.product_reachable,
                    bound_operation: spec.bound_operation.map(|s| s.to_string()),
                }
            })
            .collect();

        // Preserve the bridge's schemas as metadata. Never executable.
        for definition in crate::enhanced_bridge::EnhancedMcpBridge::new().all_definitions() {
            if entries.iter().any(|e| e.id == definition.id) {
                continue; // the real executor wins; the definition is merged into it
            }
            entries.push(ToolEntry {
                id: definition.id,
                description: definition.description,
                input_schema: definition.parameters,
                risk: RiskLevel::Execute,
                permissions: default_permissions_for(RiskLevel::Execute),
                status: CapabilityStatus::DefinitionOnly,
                rung: EvidenceRung::R1Observed,
                product_reachable: false,
                bound_operation: None,
            });
        }

        entries.sort_by(|a, b| a.id.cmp(&b.id));
        Self { entries }
    }

    pub fn entries(&self) -> &[ToolEntry] {
        &self.entries
    }

    pub fn get(&self, id: &str) -> Option<&ToolEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Ids that have a real executor. This is the only set that may be
    /// registered as executable.
    pub fn executable_ids(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|e| e.status.is_executable())
            .map(|e| e.id.as_str())
            .collect()
    }

    /// Ids preserved as metadata only. These MUST NOT be registered.
    pub fn definition_only_ids(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|e| !e.status.is_executable())
            .map(|e| e.id.as_str())
            .collect()
    }

    /// The five explicit metrics. There is intentionally no `tool_count`.
    pub fn metrics(&self) -> CatalogueMetrics {
        let executable: Vec<&ToolEntry> = self
            .entries
            .iter()
            .filter(|e| e.status.is_executable())
            .collect();

        let tested = executable
            .iter()
            .filter(|e| {
                EXECUTABLE_SPECS
                    .iter()
                    .find(|s| s.id == e.id)
                    .map(|s| s.tested)
                    .unwrap_or(false)
            })
            .count();

        CatalogueMetrics {
            definition_count: self.entries.len(),
            executable_count: executable.len(),
            tested_execution_count: tested,
            product_reachable_count: executable.iter().filter(|e| e.product_reachable).count(),
            r3_verified_count: executable
                .iter()
                .filter(|e| e.rung >= EvidenceRung::R3Verified)
                .count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_executable_entry_has_a_real_executor() {
        let catalogue = Catalogue::canonical();
        for entry in catalogue
            .entries()
            .iter()
            .filter(|e| e.status.is_executable())
        {
            assert!(
                get_real_tool(&entry.id).is_some(),
                "`{}` is marked Executable but get_real_tool() returns None",
                entry.id
            );
        }
    }

    #[test]
    fn every_definition_only_entry_has_no_executor() {
        let catalogue = Catalogue::canonical();
        for entry in catalogue
            .entries()
            .iter()
            .filter(|e| !e.status.is_executable())
        {
            assert!(
                get_real_tool(&entry.id).is_none(),
                "`{}` is marked DefinitionOnly but a real executor exists — \
                 it must be promoted to Executable, not left as metadata",
                entry.id
            );
        }
    }

    #[test]
    fn every_entry_has_a_schema_and_a_risk_class() {
        let catalogue = Catalogue::canonical();
        for entry in catalogue.entries() {
            assert!(
                entry.input_schema.is_object(),
                "`{}` has no object schema",
                entry.id
            );
            assert!(
                entry.input_schema.get("properties").is_some(),
                "`{}` schema has no `properties`",
                entry.id
            );
        }
    }

    #[test]
    fn executable_ids_are_unique() {
        let catalogue = Catalogue::canonical();
        let mut ids = catalogue.executable_ids();
        let before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            before,
            ids.len(),
            "duplicate executable id in the catalogue"
        );
    }

    #[test]
    fn executable_and_definition_only_are_disjoint_and_cover_everything() {
        let catalogue = Catalogue::canonical();
        let exec = catalogue.executable_ids().len();
        let meta = catalogue.definition_only_ids().len();
        assert_eq!(
            exec + meta,
            catalogue.metrics().definition_count,
            "every entry must be exactly one of executable / definition-only"
        );
    }

    #[test]
    fn metrics_are_separately_scoped_and_ordered() {
        let m = Catalogue::canonical().metrics();
        assert!(m.executable_count <= m.definition_count);
        assert!(
            m.tested_execution_count <= m.executable_count,
            "a tested execution must also be executable"
        );
        assert!(
            m.r3_verified_count <= m.product_reachable_count,
            "R3 requires product reachability"
        );
        assert!(
            m.product_reachable_count <= m.executable_count,
            "product reachability requires an executor"
        );
    }

    #[test]
    fn no_capability_claims_r3_yet() {
        // Guards against a future change silently promoting something to R3
        // without the commissioning evidence R3 requires.
        let m = Catalogue::canonical().metrics();
        assert_eq!(
            m.r3_verified_count, 0,
            "R3 requires a named product surface AND captured execution evidence; \
             update this test only together with docs/governance/R3_COMMISSIONING_PLAN.md"
        );
    }

    #[test]
    fn git_commit_is_executable_and_bound_to_commit_only() {
        let catalogue = Catalogue::canonical();
        let entry = catalogue.get("git.commit").expect("git.commit catalogued");
        assert!(entry.status.is_executable());
        assert_eq!(entry.bound_operation.as_deref(), Some("git commit"));
    }

    #[test]
    fn bridge_only_ids_are_definition_only() {
        let catalogue = Catalogue::canonical();
        for id in [
            "git.push",
            "git.pull",
            "npm.install",
            "docker.build",
            "kubernetes.deploy",
        ] {
            let entry = catalogue
                .get(id)
                .unwrap_or_else(|| panic!("`{id}` should be preserved as metadata"));
            assert_eq!(
                entry.status,
                CapabilityStatus::DefinitionOnly,
                "`{id}` has no real executor and must not be executable"
            );
        }
    }

    /// The shipped configuration lists exactly the executable set.
    ///
    /// This is the mechanical form of the invariant stated in
    /// `docs/governance/MCP_TOOLS_YAML_DISPOSITION.md`:
    ///
    /// ```text
    ///     { ids under `tools:` }  ==  { id | get_real_tool(id).is_some() }
    /// ```
    ///
    /// It is what stops an unsupported definition from reaching the product as
    /// callable. Before the consolidation a `DynamicTool` with no executor
    /// returned a simulated success, so such an entry appeared to work.
    #[test]
    fn shipped_config_lists_exactly_the_executable_set() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("mcp.tools.yaml");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

        #[derive(serde::Deserialize)]
        struct File {
            #[serde(default)]
            tools: Vec<Entry>,
        }
        #[derive(serde::Deserialize)]
        struct Entry {
            id: String,
        }

        let file: File = serde_yaml::from_str(&raw).expect("mcp.tools.yaml must parse");
        let configured: std::collections::BTreeSet<&str> =
            file.tools.iter().map(|t| t.id.as_str()).collect();

        let catalogue = Catalogue::canonical();
        let executable: std::collections::BTreeSet<&str> =
            catalogue.executable_ids().into_iter().collect();

        let missing: Vec<&&str> = executable.difference(&configured).collect();
        let extra: Vec<&&str> = configured.difference(&executable).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "mcp.tools.yaml `tools:` must equal the executable set exactly.\n\
             missing from the config: {missing:?}\n\
             present but not executable: {extra:?}"
        );
    }

    /// Nothing without an executor may be listed as a tool in the shipped config.
    #[test]
    fn shipped_config_has_no_definition_only_tools() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("mcp.tools.yaml");
        let raw = std::fs::read_to_string(&path).expect("mcp.tools.yaml readable");

        #[derive(serde::Deserialize)]
        struct File {
            #[serde(default)]
            tools: Vec<Entry>,
        }
        #[derive(serde::Deserialize)]
        struct Entry {
            id: String,
        }

        let file: File = serde_yaml::from_str(&raw).expect("mcp.tools.yaml must parse");
        for entry in &file.tools {
            assert!(
                get_real_tool(&entry.id).is_some(),
                "`{}` is listed as executable but has no real executor",
                entry.id
            );
        }
    }

    /// Pins the post-consolidation counts.
    ///
    /// These figures appear in `docs/governance/TOOL_CATALOGUE_TRUTH_TABLE.md`
    /// §11.5. Pinning them here means the documents cannot drift from the code:
    /// a change to any count fails this test until the record is updated in the
    /// same commit.
    #[test]
    fn metrics_match_the_governance_record() {
        let m = Catalogue::canonical().metrics();
        assert_eq!(
            m.definition_count, 38,
            "12 executable + 27 bridge definitions - 1 overlap (git.commit)"
        );
        assert_eq!(m.executable_count, 12);
        assert_eq!(
            m.tested_execution_count, 2,
            "only fs.read and shell.execute are exercised by committed tests"
        );
        assert_eq!(
            m.product_reachable_count, 11,
            "git.commit is not product-reachable"
        );
        assert_eq!(m.r3_verified_count, 0);
    }

    fn test_context() -> crate::real_tools::ToolContext {
        crate::real_tools::ToolContext {
            working_directory: std::path::PathBuf::from("."),
            environment: std::collections::HashMap::new(),
            timeout: std::time::Duration::from_secs(5),
            session_id: None,
            actor: Some("catalogue-test".to_string()),
            approval_required: false,
        }
    }

    /// The catalogue's risk classes and the default gate agree.
    ///
    /// A catalogue that classifies risk while the gate ignores the
    /// classification would have documented a policy and implemented none of
    /// it. This is the test that keeps the two in step.
    #[test]
    fn default_gate_matches_the_catalogue_risk_classes() {
        let catalogue = Catalogue::canonical();
        let gate = crate::permission::PermissionGate::restrictive();

        for entry in catalogue
            .entries()
            .iter()
            .filter(|e| e.status.is_executable())
        {
            let decision = gate.decide(&entry.id, entry.risk, &test_context());
            if entry.risk == RiskLevel::Read {
                assert!(
                    decision.is_allow(),
                    "`{}` is Read and should be permitted by the default gate, got {decision:?}",
                    entry.id
                );
            } else {
                assert!(
                    !decision.is_allow(),
                    "`{}` is {:?} and must require approval by default, got {decision:?}",
                    entry.id,
                    entry.risk
                );
            }
        }
    }

    /// The one unbound escape hatch is gated at the highest practical class.
    #[test]
    fn the_escape_hatch_is_gated() {
        let catalogue = Catalogue::canonical();
        let entry = catalogue.get("shell.execute").expect("catalogued");
        assert_eq!(entry.risk, RiskLevel::Execute);
        assert_eq!(
            entry.bound_operation, None,
            "shell.execute is unbound in the catalogue"
        );
        assert_eq!(
            get_real_tool("shell.execute").unwrap().bound_operation(),
            None,
            "the shell.execute executor must be unbound"
        );

        let gate = crate::permission::PermissionGate::restrictive();
        assert!(
            !gate
                .decide(&entry.id, entry.risk, &test_context())
                .is_allow(),
            "the unbound escape hatch must not run unapproved"
        );
        // An explicit allow-list entry is the stated way to permit it.
        let gate = crate::permission::PermissionGate::new(
            crate::permission::PermissionPolicy::restrictive().allow("shell.execute"),
        );
        assert!(gate
            .decide(&entry.id, entry.risk, &test_context())
            .is_allow());
    }

    /// The catalogue's declared binding equals the executor's actual binding.
    ///
    /// A catalogue that describes a binding the executor does not enforce would
    /// be a claim, not a control. `None` means unbound in both.
    #[test]
    fn catalogue_binding_matches_the_executor_binding() {
        let catalogue = Catalogue::canonical();
        for entry in catalogue
            .entries()
            .iter()
            .filter(|e| e.status.is_executable())
        {
            let executor = get_real_tool(&entry.id)
                .unwrap_or_else(|| panic!("no executor for `{}`", entry.id));
            assert_eq!(
                entry.bound_operation,
                executor.bound_operation(),
                "`{}`: the catalogue and the executor disagree about the binding",
                entry.id
            );
            assert_eq!(
                entry.risk,
                executor.risk_level(),
                "`{}`: the catalogue and the executor disagree about the risk class",
                entry.id
            );
        }
    }

    /// Every executable entry declares a non-empty permission set.
    #[test]
    fn every_executable_entry_declares_permissions() {
        let catalogue = Catalogue::canonical();
        for entry in catalogue
            .entries()
            .iter()
            .filter(|e| e.status.is_executable())
        {
            let p = &entry.permissions;
            let any = p.read || p.write || p.execute || p.network || p.destructive;
            assert!(any, "`{}` declares no permissions at all", entry.id);
            // A read-class tool must not carry write permission.
            if entry.risk == RiskLevel::Read {
                assert!(!p.write, "`{}` is Read but declares write", entry.id);
                assert!(!p.execute, "`{}` is Read but declares execute", entry.id);
            }
        }
    }
}
