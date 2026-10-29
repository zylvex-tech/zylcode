//! Mission specification: parsing, validation, and the plan built from it.
//!
//! Extracted from the mission runner so the *contract* a spec must satisfy —
//! what parses, what is rejected, what a plan looks like — can be read and
//! tested without the 2 000 lines of execution machinery that consume it.
//! Everything here is transport-independent: the CLI, the desktop and the
//! tests all parse with [`parse_spec`].
//!

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::sha256_hex;

// ---------------------------------------------------------------------------
// Specification
// ---------------------------------------------------------------------------

/// Machine-readable mission specification, embedded in a human markdown file
/// as a fenced ```json block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionSpec {
    pub intent: String,
    pub requirements: Vec<String>,
    /// Command argv executed with the mission workspace as cwd.
    pub verify_command: Vec<String>,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operation {
    /// Create/overwrite a file in the mission workspace.
    Write { path: String, content: String },
    /// Run the verification command and check exit code + output signatures.
    Run {
        id: String,
        expect: Expect,
        #[serde(default)]
        signature: Vec<String>,
    },
    /// Replace `old` with `new` exactly once in `path`.
    Replace {
        path: String,
        old: String,
        new: String,
    },
    /// Commit the current workspace state.
    Commit { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expect {
    Pass,
    Fail,
}

impl Operation {
    pub fn describe(&self) -> String {
        match self {
            Operation::Write { path, .. } => format!("write {path}"),
            Operation::Run { id, expect, .. } => format!("run '{id}' (expect {expect:?})"),
            Operation::Replace { path, .. } => format!("repair {path}"),
            Operation::Commit { message } => format!("commit: {message}"),
        }
    }
}

fn safe_rel_path(p: &str) -> Result<()> {
    let path = Path::new(p);
    if p.trim().is_empty() || path.is_absolute() || p.contains("..") {
        bail!("spec path '{p}' must be a non-empty relative path without '..'");
    }
    Ok(())
}

/// Parse the human specification: markdown prose + fenced JSON block.
/// Returns the parsed spec and the sha256 of the raw specification text.
pub fn parse_spec(text: &str) -> Result<(MissionSpec, String)> {
    let spec_hash = sha256_hex(text.as_bytes());
    let start = text
        .find("```json")
        .context("specification has no ```json block")?
        + "```json".len();
    let rest = &text[start..];
    let end = rest
        .find("```")
        .context("specification json block is unterminated")?;
    let json = &rest[..end];
    let spec: MissionSpec = serde_json::from_str(json)
        .with_context(|| format!("specification json block is invalid: {json:.200}"))?;

    if spec.intent.trim().is_empty() {
        bail!("spec intent must not be empty");
    }
    if spec.verify_command.is_empty() {
        bail!("spec verify_command must not be empty");
    }
    if spec.operations.is_empty() {
        bail!("spec operations must not be empty");
    }
    for op in &spec.operations {
        match op {
            Operation::Write { path, content } => {
                safe_rel_path(path)?;
                if content.is_empty() {
                    bail!("write op for '{path}' has empty content");
                }
            }
            Operation::Replace { path, old, new } => {
                safe_rel_path(path)?;
                if old.is_empty() || new.is_empty() || old == new {
                    bail!("replace op for '{path}' needs distinct non-empty old/new");
                }
            }
            Operation::Run { id, signature, .. } => {
                if id.trim().is_empty() {
                    bail!("run op needs a non-empty id");
                }
                let _ = signature;
            }
            Operation::Commit { message } => {
                if message.trim().is_empty() {
                    bail!("commit op needs a message");
                }
            }
        }
    }
    Ok((spec, spec_hash))
}

// ---------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub schema_version: u32,
    pub created_at: DateTime<Utc>,
    pub steps: Vec<PlanStep>,
}

pub(crate) fn build_plan(spec: &MissionSpec, fixture_name: &str) -> Plan {
    let mut steps = vec![
        PlanStep {
            id: "prepare".into(),
            description: format!("Copy fixture '{fixture_name}' into the mission workspace and create the baseline git revision"),
        },
        PlanStep {
            id: "baseline".into(),
            description: "Run the verification command; the suite must pass before any modification".into(),
        },
    ];
    for (i, op) in spec.operations.iter().enumerate() {
        steps.push(PlanStep {
            id: format!("op:{i}"),
            description: op.describe(),
        });
    }
    steps.push(PlanStep {
        id: "finalize".into(),
        description: "Classify claims, verify evidence integrity, write the engineering record, export the reproducibility package".into(),
    });
    Plan {
        schema_version: 1,
        created_at: Utc::now(),
        steps,
    }
}
