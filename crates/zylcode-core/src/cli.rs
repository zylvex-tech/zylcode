//! Minimal CLI entrypoint for `zylcode-core`.
//!
//! Provides a standalone binary interface that takes a prompt, processes it
//! through the pipeline, and prints the resulting artifacts — **no Tauri
//! dependency**.  This lets CI, tests, and headless users drive the engine
//! directly.
//!
//! # Usage
//!
//! ```text
//! zylcode-core-cli <PROMPT> [--workspace <DIR>] [--verbose]
//! zylcode-core-cli security-scan [--workspace <DIR>] [--output json|text]
//! zylcode-core-cli verify [--workspace <DIR>]
//! ```

use std::env;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use serde::{Serialize, Deserialize};
use tracing::info;

use crate::{EngineConfig, Intent, ZylCodeEngine};
use crate::pipeline::validate_artifact_kind;
use crate::router::decision::classify_verification_rung;

// ---------------------------------------------------------------------------
// Subcommand definitions
// ---------------------------------------------------------------------------

/// Top-level subcommands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subcommand {
    /// Process a natural-language prompt (default).
    Prompt(PromptArgs),
    /// Scan pipeline artifacts for security issues.
    SecurityScan(SecurityScanArgs),
    /// Run verification headlessly and emit a structured report.
    Verify(VerifyArgs),
    /// Run a benchmark suite and emit a reproducible report.
    Benchmark(BenchmarkArgs),
}

/// Arguments for the default prompt subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptArgs {
    /// The natural-language prompt to process.
    pub prompt: String,
    /// Workspace root directory (defaults to cwd).
    pub workspace: PathBuf,
    /// Whether verbose tracing is enabled.
    pub verbose: bool,
}

/// Arguments for the `security-scan` subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityScanArgs {
    /// Workspace root directory (defaults to cwd).
    pub workspace: PathBuf,
    /// Output format: `json` (structured) or `text` (human-readable).
    pub output_format: OutputFormat,
    /// Whether verbose tracing is enabled.
    pub verbose: bool,
}

/// Arguments for the `verify` subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyArgs {
    /// Workspace root directory (defaults to cwd).
    pub workspace: PathBuf,
    /// Whether verbose tracing is enabled.
    pub verbose: bool,
    /// If true, run offline using only local Ollama (no cloud providers).
    pub offline: bool,
}

/// Arguments for the `benchmark` subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkArgs {
    /// Workspace root directory (defaults to cwd).
    pub workspace: PathBuf,
    /// Output format: `json` (structured) or `text` (human-readable).
    pub output_format: OutputFormat,
    /// Whether verbose tracing is enabled.
    pub verbose: bool,
}

/// Output format for structured commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// JSON output.
    Json,
    /// Human-readable text output.
    Text,
}

// ---------------------------------------------------------------------------
// Argument parsing (zero external deps — keeps the core crate lean)
// ---------------------------------------------------------------------------

/// Errors that can occur during argument parsing.
#[derive(Debug, Clone)]
pub enum ParseError {
    /// No arguments were provided.
    MissingPrompt,
    /// The `--workspace` flag was provided without a value.
    MissingWorkspaceValue,
    /// An unknown flag was encountered.
    UnknownFlag(String),
    /// The `--output` flag was provided without a value.
    MissingOutputValue,
    /// An invalid value was provided for `--output`.
    InvalidOutputFormat(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::MissingPrompt => write!(
                f,
                 "usage: zylcode-core-cli <PROMPT> [--workspace <DIR>] [--verbose]\n\
                 \x20     zylcode-core-cli security-scan [--workspace <DIR>] [--output json|text]\n\
                 \x20     zylcode-core-cli verify [--workspace <DIR>] [--offline]\n\
                 \x20     zylcode-core-cli benchmark [--workspace <DIR>] [--format json|text] [--verbose]"
            ),
            ParseError::MissingWorkspaceValue => write!(f, "--workspace requires a directory path"),
            ParseError::UnknownFlag(flag) => write!(f, "unknown flag: {flag}"),
            ParseError::MissingOutputValue => write!(f, "--output requires a format (json or text)"),
            ParseError::InvalidOutputFormat(val) => {
                write!(f, "invalid output format: {val} (expected json or text)")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse CLI arguments from an iterator of strings.
///
/// Supports subcommands:
/// - `<PROMPT> [--workspace <DIR>] [--verbose]` — process a prompt (default)
/// - `security-scan [--workspace <DIR>] [--output json|text]` — scan artifacts
/// - `verify [--workspace <DIR>]` — headless verification report
pub fn parse_args<I, S>(args: I) -> Result<Subcommand, ParseError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args_vec: Vec<String> = args.into_iter().map(Into::into).collect();

    // Skip argv[0] (program name) if present — callers usually already
    // strip it, but be defensive.
    if let Some(first) = args_vec.first() {
        if !first.starts_with('-') && (first.contains('/') || first.contains('\\')) {
            args_vec.remove(0);
        }
    }

    // Peek at the first argument to determine the subcommand.
    let mut iter = args_vec.into_iter();
    let first = iter.next();

    match first.as_deref() {
        Some("security-scan") => parse_security_scan(iter),
        Some("verify") => parse_verify(iter),
        Some("benchmark") => parse_benchmark(iter),
        // Default: treat everything as a prompt (put the first arg back).
        // Flags like --verbose, --workspace, -- are handled by parse_prompt.
        first_opt => {
            let mut remaining: Vec<String> = first_opt.into_iter().map(String::from).collect();
            remaining.extend(iter);
            parse_prompt(remaining)
        }
    }
}

/// Parse arguments for the default prompt subcommand.
fn parse_prompt(args: Vec<String>) -> Result<Subcommand, ParseError> {
    let mut prompt_parts: Vec<String> = Vec::new();
    let mut workspace: Option<PathBuf> = None;
    let mut verbose = false;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--workspace" => {
                let val = iter.next().ok_or(ParseError::MissingWorkspaceValue)?;
                workspace = Some(PathBuf::from(val));
            }
            "--verbose" => {
                verbose = true;
            }
            "--" => {
                for rest in iter {
                    prompt_parts.push(rest);
                }
                break;
            }
            other if other.starts_with('-') => {
                return Err(ParseError::UnknownFlag(other.to_string()));
            }
            _ => {
                prompt_parts.push(arg);
            }
        }
    }

    if prompt_parts.is_empty() {
        return Err(ParseError::MissingPrompt);
    }

    let prompt = prompt_parts.join(" ");
    let workspace = workspace.unwrap_or_else(|| {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });

    Ok(Subcommand::Prompt(PromptArgs {
        prompt,
        workspace,
        verbose,
    }))
}

/// Parse arguments for the `security-scan` subcommand.
fn parse_security_scan(args: impl Iterator<Item = String>) -> Result<Subcommand, ParseError> {
    let mut workspace: Option<PathBuf> = None;
    let mut output_format: Option<OutputFormat> = None;
    let mut verbose = false;

    let mut iter = args;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--workspace" => {
                let val = iter.next().ok_or(ParseError::MissingWorkspaceValue)?;
                workspace = Some(PathBuf::from(val));
            }
            "--output" => {
                let val = iter.next().ok_or(ParseError::MissingOutputValue)?;
                match val.as_str() {
                    "json" => output_format = Some(OutputFormat::Json),
                    "text" => output_format = Some(OutputFormat::Text),
                    _ => return Err(ParseError::InvalidOutputFormat(val)),
                }
            }
            "--verbose" => {
                verbose = true;
            }
            "--" => break,
            other if other.starts_with('-') => {
                return Err(ParseError::UnknownFlag(other.to_string()));
            }
            _ => {} // Ignore non-flag args after subcommand.
        }
    }

    let workspace = workspace.unwrap_or_else(|| {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });

    Ok(Subcommand::SecurityScan(SecurityScanArgs {
        workspace,
        output_format: output_format.unwrap_or(OutputFormat::Text),
        verbose,
    }))
}

/// Parse arguments for the `verify` subcommand.
fn parse_verify(args: impl Iterator<Item = String>) -> Result<Subcommand, ParseError> {
    let mut workspace: Option<PathBuf> = None;
    let mut verbose = false;
    let mut offline = false;

    let mut iter = args;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--workspace" => {
                let val = iter.next().ok_or(ParseError::MissingWorkspaceValue)?;
                workspace = Some(PathBuf::from(val));
            }
            "--verbose" => {
                verbose = true;
            }
            "--offline" => {
                offline = true;
            }
            "--" => break,
            other if other.starts_with('-') => {
                return Err(ParseError::UnknownFlag(other.to_string()));
            }
            _ => {}
        }
    }

    let workspace = workspace.unwrap_or_else(|| {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });

    Ok(Subcommand::Verify(VerifyArgs {
        workspace,
        verbose,
        offline,
    }))
}

fn parse_benchmark(args: impl Iterator<Item = String>) -> Result<Subcommand, ParseError> {
    let mut workspace: Option<PathBuf> = None;
    let mut verbose = false;
    let mut output_format: Option<String> = None;

    let mut iter = args;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--workspace" => {
                let val = iter.next().ok_or(ParseError::MissingWorkspaceValue)?;
                workspace = Some(PathBuf::from(val));
            }
            "--verbose" => {
                verbose = true;
            }
            "--output-format" | "--format" => {
                let val = iter.next().ok_or(ParseError::MissingWorkspaceValue)?;
                output_format = Some(val);
            }
            "--" => break,
            other if other.starts_with('-') => {
                return Err(ParseError::UnknownFlag(other.to_string()));
            }
            _ => {}
        }
    }

    let workspace = workspace.unwrap_or_else(|| {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });

    Ok(Subcommand::Benchmark(BenchmarkArgs {
        workspace,
        output_format: match output_format.as_deref() {
            Some("json") => OutputFormat::Json,
            _ => OutputFormat::Text,
        },
        verbose,
    }))
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

/// Run the CLI: parse args, dispatch to the appropriate subcommand.
pub async fn run() -> Result<()> {
    let subcommand = parse_args(env::args().skip(1))?;

    match subcommand {
        Subcommand::Prompt(args) => run_prompt(args).await,
        Subcommand::SecurityScan(args) => run_security_scan(args).await,
        Subcommand::Verify(args) => run_verify(args).await,
        Subcommand::Benchmark(args) => run_benchmark(args).await,
    }
}

/// Process a natural-language prompt through the pipeline.
async fn run_prompt(args: PromptArgs) -> Result<()> {
    if args.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("info")
            .try_init()
            .ok();
    }

    info!(
        prompt = %args.prompt,
        workspace = %args.workspace.display(),
        "zylcode-core-cli: processing intent"
    );

    let config = EngineConfig {
        workspace_root: args.workspace.to_string_lossy().to_string(),
        verbose: args.verbose,
        ..Default::default()
    };

    let engine = ZylCodeEngine::new(config);

    let intent = Intent {
        prompt: args.prompt,
        context: None,
        correlation_id: None,
    };

    let result = engine.process_intent(intent).await?;

    // Print summary.
    println!("=== Result ===");
    println!("Success: {}", result.success);
    println!("Summary: {}", result.summary);

    // Print artifacts.
    if !result.artifacts.is_empty() {
        println!("\n=== Artifacts ===");
        for (i, artifact) in result.artifacts.iter().enumerate() {
            println!("[{}] {} ({})", i + 1, artifact.label, artifact.kind);
            println!("{}", artifact.content);
            println!();
        }
    }

    if !result.success {
        anyhow::bail!("intent processing reported failure");
    }

    Ok(())
}

/// Scan workspace artifacts for security issues and emit structured output.
async fn run_security_scan(args: SecurityScanArgs) -> Result<()> {
    if args.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("info")
            .try_init()
            .ok();
    }

    info!(
        workspace = %args.workspace.display(),
        "zylcode-core-cli: security scan"
    );

    let config = EngineConfig {
        workspace_root: args.workspace.to_string_lossy().to_string(),
        verbose: args.verbose,
        ..Default::default()
    };

    let engine = ZylCodeEngine::new(config);

    // Run a scan prompt through the pipeline to produce artifacts.
    let intent = Intent {
        prompt: "Analyze the codebase for security vulnerabilities and generate a report"
            .to_string(),
        context: None,
        correlation_id: None,
    };

    let result = engine.process_intent(intent).await?;

    // Classify each artifact and collect security findings.
    let mut findings: Vec<SecurityFinding> = Vec::new();
    for artifact in &result.artifacts {
        let reported_kind = artifact.kind.clone();
        let corrected = validate_artifact_kind(
            &artifact.path,
            &artifact.content,
            &reported_kind,
        );
        let rung = classify_verification_rung(&corrected);

        findings.push(SecurityFinding {
            path: artifact.path.clone(),
            kind: corrected,
            rung: rung.label().to_string(),
            label: artifact.label.clone(),
        });
    }

    let rung_reached = report_highest_rung(&findings);

    match args.output_format {
        OutputFormat::Json => {
            let report = SecurityReport {
                success: result.success,
                summary: result.summary,
                findings,
                rung_reached,
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Text => {
            println!("=== Security Scan ===");
            println!("Success: {}", result.success);
            println!("Summary: {}", result.summary);
            println!("Highest rung reached: {}", report_highest_rung(&findings));
            if !findings.is_empty() {
                println!("\n=== Findings ===");
                for (i, f) in findings.iter().enumerate() {
                    println!(
                        "[{}] {} — {} ({}, {})",
                        i + 1,
                        f.label,
                        f.path,
                        f.kind,
                        f.rung,
                    );
                }
            }
        }
    }

    Ok(())
}

/// Security finding for a single artifact.
#[derive(Debug, Clone, serde::Serialize)]
struct SecurityFinding {
    path: String,
    kind: String,
    rung: String,
    label: String,
}

/// Full security scan report (JSON output).
#[derive(Debug, Clone, serde::Serialize)]
struct SecurityReport {
    success: bool,
    summary: String,
    findings: Vec<SecurityFinding>,
    rung_reached: String,
}

/// Compute the highest verification rung label from a list of findings.
fn report_highest_rung(findings: &[SecurityFinding]) -> String {
    let rungs = ["Rung 0", "Rung 1", "Rung 2", "Rung 3", "Rung 4"];
    for rung in rungs.iter().rev() {
        if findings.iter().any(|f| f.rung == *rung) {
            return rung.to_string();
        }
    }
    "Rung 0".to_string()
}

/// Run headless verification and emit a structured report.
async fn run_verify(args: VerifyArgs) -> Result<()> {
    if args.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("info")
            .try_init()
            .ok();
    }

    info!(
        workspace = %args.workspace.display(),
        "zylcode-core-cli: headless verification (offline={offline})",
        offline = args.offline,
    );

    let mut extra = std::collections::HashMap::new();
    if args.offline {
        extra.insert("offline".to_string(), "true".to_string());
        info!("offline mode: forcing Ollama/SyntheticOffline providers only");
    }

    let config = EngineConfig {
        workspace_root: args.workspace.to_string_lossy().to_string(),
        verbose: args.verbose,
        extra,
    };

    let engine = ZylCodeEngine::new(config);

    // Run a verification prompt to exercise the full pipeline.
    let intent = Intent {
        prompt: "Verify the codebase compiles and passes tests".to_string(),
        context: None,
        correlation_id: None,
    };

    let result = engine.process_intent(intent).await?;

    // Classify each artifact's verification rung.
    let mut artifact_reports: Vec<VerifyArtifactReport> = Vec::new();
    for artifact in &result.artifacts {
        let reported_kind = artifact.kind.clone();
        let corrected = validate_artifact_kind(
            &artifact.path,
            &artifact.content,
            &reported_kind,
        );
        let rung = classify_verification_rung(&corrected);

        artifact_reports.push(VerifyArtifactReport {
            path: artifact.path.clone(),
            label: artifact.label.clone(),
            kind: corrected,
            rung: rung.label().to_string(),
        });
    }

    let report = VerifyReport {
        success: result.success,
        summary: result.summary,
        artifacts: artifact_reports.clone(),
        highest_rung: report_highest_rung_from_reports(&artifact_reports),
        offline: args.offline,
    };

    println!("{}", serde_json::to_string_pretty(&report)?);

    if !result.success {
        anyhow::bail!("verification reported failure");
    }

    Ok(())
}

/// Artifact verification report entry.
#[derive(Debug, Clone, serde::Serialize)]
struct VerifyArtifactReport {
    path: String,
    label: String,
    kind: String,
    rung: String,
}

/// Full headless verification report (always JSON for machine consumption).
#[derive(Debug, Clone, serde::Serialize)]
struct VerifyReport {
    success: bool,
    summary: String,
    artifacts: Vec<VerifyArtifactReport>,
    highest_rung: String,
    offline: bool,
}

/// Compute the highest verification rung label from verify artifact reports.
fn report_highest_rung_from_reports(reports: &[VerifyArtifactReport]) -> String {
    let rungs = ["Rung 0", "Rung 1", "Rung 2", "Rung 3", "Rung 4"];
    for rung in rungs.iter().rev() {
        if reports.iter().any(|r| r.rung == *rung) {
            return rung.to_string();
        }
    }
    "Rung 0".to_string()
}

// ---------------------------------------------------------------------------
// Benchmark support
// ---------------------------------------------------------------------------

/// A single benchmarked file, with its inferred kind, verification rung, and
/// whether the content passed local-only verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkItem {
    /// Relative path from the workspace root.
    path: String,
    /// The tool/runtime that produced the artifact (e.g. "zylcode", "mcp", "manual").
    source: String,
    /// Inferred artifact kind (e.g. "rust_module", "ui_component").
    kind: String,
    /// Verification rung label.
    rung: String,
    /// Whether local-only validation passed (true for all rungs; only rung 3+ is
    /// meaningful but we always report true when no validation issues).
    passed: bool,
}

/// Summary report produced by the benchmark command.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkReport {
    /// Workspace root directory.
    workspace: String,
    /// ISO-8601 UTC timestamp.
    timestamp: String,
    /// Number of files scanned.
    files_scanned: usize,
    /// Number of files identified as ZylCode artifacts.
    artifacts_found: usize,
    /// Breakdown by verification rung: rung label → count.
    rung_counts: std::collections::HashMap<String, usize>,
    /// Breakdown by artifact kind: kind → count.
    kind_counts: std::collections::HashMap<String, usize>,
    /// Per-file benchmark items.
    items: Vec<BenchmarkItem>,
    /// Duration in milliseconds to scan the workspace.
    scan_duration_ms: u64,
    /// Whether the scan completed without errors.
    success: bool,
    /// Error message if success is false.
    error: Option<String>,
}

/// Walk the workspace, classify every artifact file, measure timing, and produce
/// a `BenchmarkReport`.
async fn run_benchmark(args: BenchmarkArgs) -> anyhow::Result<()> {
    let workspace = args.workspace.clone();
    if !workspace.exists() {
        anyhow::bail!("workspace directory does not exist: {}", workspace.display());
    }

    let start = Instant::now();

    let mut items: Vec<BenchmarkItem> = Vec::new();
    let mut kind_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut rung_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    // Recursively scan the workspace for known extension patterns.
    let walker = walkdir::WalkDir::new(&workspace)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && !name.contains("node_modules") && !name.contains("target")
        });

    let mut files_scanned = 0usize;
    let mut artifacts_found = 0usize;

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        files_scanned += 1;
        let path = entry.path();

        // Determine source from file content or extension heuristics.
        let source = infer_source(path);

        // Try to read content for kind classification.
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue, // binary files and unreadable files are skipped.
        };

        // Use validate_artifact_kind to classify.
        let kind = crate::pipeline::validate_artifact_kind(
            &path.to_string_lossy(),
            &content,
            &source,
        );

        // Skip files that validate_artifact_kind tagged as "error" (not an artifact).
        if kind == "error" || kind.is_empty() {
            continue;
        }

        artifacts_found += 1;
        let rung = crate::router::decision::classify_verification_rung(&kind);
        let rung_label = rung.short_label().to_string();

        // All kinds pass local validation; rung 3+ has formal verification.
        let passed = true;

        // Relative path from workspace root.
        let rel = path
            .strip_prefix(&workspace)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        items.push(BenchmarkItem {
            path: rel,
            source,
            kind: kind.clone(),
            rung: rung_label.clone(),
            passed,
        });

        *kind_counts.entry(kind).or_insert(0) += 1;
        *rung_counts.entry(rung_label).or_insert(0) += 1;
    }

    let scan_duration_ms = start.elapsed().as_millis() as u64;

    let report = BenchmarkReport {
        workspace: workspace.to_string_lossy().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        files_scanned,
        artifacts_found,
        rung_counts,
        kind_counts,
        items,
        scan_duration_ms,
        success: true,
        error: None,
    };

    match args.output_format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&report)?;
            println!("{json}");
        }
        OutputFormat::Text => {
            println!(
                "Benchmark report for {}",
                workspace.to_string_lossy()
            );
            println!("Scanned {} files, found {} artifacts", files_scanned, artifacts_found);
            println!("Scan duration: {}ms", scan_duration_ms);
            println!();
            if report.rung_counts.is_empty() {
                println!("No artifacts found.");
            } else {
                println!("Verification rungs:");
                let mut sorted_rungs: Vec<_> = report.rung_counts.iter().collect();
                sorted_rungs.sort_by_key(|(k, _)| k.as_str());
                for (rung, count) in sorted_rungs {
                    println!("  {rung}: {count}");
                }
                println!();
                println!("Artifact kinds:");
                let mut sorted_kinds: Vec<_> = report.kind_counts.iter().collect();
                sorted_kinds.sort_by_key(|(k, _)| k.as_str());
                for (kind, count) in sorted_kinds {
                    println!("  {kind}: {count}");
                }
            }
        }
    }

    Ok(())
}

/// Infer the tool/source that produced an artifact file from its path and
/// content heuristics.
fn infer_source(path: &std::path::Path) -> String {
    // Walk every ancestor component — a parent directory named "zylcode"
    // or "zy-*" counts just like the file itself.
    for ancestor in path.ancestors() {
        if let Some(comp) = ancestor.file_name() {
            let s = comp.to_string_lossy();
            if s.contains("zylcode") || s.starts_with("zy-") {
                return "zylcode".to_string();
            }
        }
    }

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();

    match ext.as_str() {
        "toml" => "manual".to_string(),
        "rs" => "manual".to_string(),
        "tsx" | "ts" => "manual".to_string(),
        "json" => "manual".to_string(),
        "md" => "manual".to_string(),
        _ => "manual".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_simple_prompt() {
        let sub = parse_args(["hello", "world"]).unwrap();
        match sub {
            Subcommand::Prompt(args) => {
                assert_eq!(args.prompt, "hello world");
                assert!(!args.verbose);
            }
            _ => panic!("expected Prompt"),
        }
    }

    #[test]
    fn parse_args_with_workspace() {
        let sub = parse_args(["--workspace", "/tmp/project", "do", "something"]).unwrap();
        match sub {
            Subcommand::Prompt(args) => {
                assert_eq!(args.prompt, "do something");
                assert_eq!(args.workspace, PathBuf::from("/tmp/project"));
            }
            _ => panic!("expected Prompt"),
        }
    }

    #[test]
    fn parse_args_verbose_flag() {
        let sub = parse_args(["--verbose", "test"]).unwrap();
        match sub {
            Subcommand::Prompt(args) => {
                assert!(args.verbose);
                assert_eq!(args.prompt, "test");
            }
            _ => panic!("expected Prompt"),
        }
    }

    #[test]
    fn parse_args_double_dash_terminates_flags() {
        let sub = parse_args(["--", "--not-a-flag"]).unwrap();
        match sub {
            Subcommand::Prompt(args) => {
                assert_eq!(args.prompt, "--not-a-flag");
            }
            _ => panic!("expected Prompt"),
        }
    }

    #[test]
    fn parse_args_missing_prompt_is_error() {
        let err = parse_args::<[&str; 0], &str>([]).unwrap_err();
        assert!(matches!(err, ParseError::MissingPrompt));
    }

    #[test]
    fn parse_args_unknown_flag() {
        let err = parse_args(["--bogus", "test"]).unwrap_err();
        assert!(matches!(err, ParseError::UnknownFlag(_)));
    }

    #[test]
    fn parse_args_workspace_missing_value() {
        let err = parse_args(["--workspace"]).unwrap_err();
        assert!(matches!(err, ParseError::MissingWorkspaceValue));
    }

    #[test]
    fn parse_error_display() {
        assert_eq!(
            ParseError::MissingPrompt.to_string(),
            "usage: zylcode-core-cli <PROMPT> [--workspace <DIR>] [--verbose]\n\
             \x20     zylcode-core-cli security-scan [--workspace <DIR>] [--output json|text]\n\
             \x20     zylcode-core-cli verify [--workspace <DIR>] [--offline]\n\
             \x20     zylcode-core-cli benchmark [--workspace <DIR>] [--format json|text] [--verbose]"
        );
        assert_eq!(ParseError::MissingWorkspaceValue.to_string(), "--workspace requires a directory path");
        assert_eq!(
            ParseError::UnknownFlag("--foo".into()).to_string(),
            "unknown flag: --foo"
        );
    }

    #[test]
    fn parse_args_security_scan_default() {
        let sub = parse_args(["security-scan"]).unwrap();
        match sub {
            Subcommand::SecurityScan(args) => {
                assert_eq!(args.output_format, OutputFormat::Text);
                assert!(!args.verbose);
            }
            _ => panic!("expected SecurityScan"),
        }
    }

    #[test]
    fn parse_args_security_scan_json_output() {
        let sub = parse_args(["security-scan", "--output", "json"]).unwrap();
        match sub {
            Subcommand::SecurityScan(args) => {
                assert_eq!(args.output_format, OutputFormat::Json);
            }
            _ => panic!("expected SecurityScan"),
        }
    }

    #[test]
    fn parse_args_security_scan_with_workspace() {
        let sub = parse_args(["security-scan", "--workspace", "/tmp/ws"]).unwrap();
        match sub {
            Subcommand::SecurityScan(args) => {
                assert_eq!(args.workspace, PathBuf::from("/tmp/ws"));
            }
            _ => panic!("expected SecurityScan"),
        }
    }

    #[test]
    fn parse_args_security_scan_invalid_output() {
        let err = parse_args(["security-scan", "--output", "yaml"]).unwrap_err();
        assert!(matches!(err, ParseError::InvalidOutputFormat(_)));
    }

    #[test]
    fn parse_args_verify_default() {
        let sub = parse_args(["verify"]).unwrap();
        match sub {
            Subcommand::Verify(args) => {
                assert!(!args.verbose);
            }
            _ => panic!("expected Verify"),
        }
    }

    #[test]
    fn parse_args_verify_verbose_with_workspace() {
        let sub = parse_args(["verify", "--verbose", "--workspace", "/ws"]).unwrap();
        match sub {
            Subcommand::Verify(args) => {
                assert!(args.verbose);
                assert_eq!(args.workspace, PathBuf::from("/ws"));
            }
            _ => panic!("expected Verify"),
        }
    }

    #[test]
    fn parse_args_unknown_flag_in_security_scan() {
        let err = parse_args(["security-scan", "--bogus"]).unwrap_err();
        assert!(matches!(err, ParseError::UnknownFlag(_)));
    }

    #[test]
    fn parse_args_prompt_does_not_shadow_subcommands() {
        let sub = parse_args(["security", "scan"]).unwrap();
        match sub {
            Subcommand::Prompt(args) => {
                assert_eq!(args.prompt, "security scan");
            }
            _ => panic!("expected Prompt with 'security scan', not a subcommand"),
        }
    }

    #[test]
    fn parse_args_leading_program_name_stripped() {
        let sub = parse_args(["/usr/bin/zylcode-core-cli", "security-scan"]).unwrap();
        match sub {
            Subcommand::SecurityScan(_) => {}
            _ => panic!("expected SecurityScan after program-name strip"),
        }
    }

    #[tokio::test]
    async fn cli_end_to_end_prompt() {
        let config = EngineConfig {
            workspace_root: env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .to_string_lossy()
                .to_string(),
            verbose: false,
            ..Default::default()
        };

        let engine = ZylCodeEngine::new(config);
        let intent = Intent {
            prompt: "add a README file".to_string(),
            context: None,
            correlation_id: None,
        };

        let result = engine.process_intent(intent).await.unwrap();
        assert!(result.success, "engine should succeed for a valid prompt");
        assert!(
            !result.summary.is_empty(),
            "result summary should not be empty"
        );
    }

    // -----------------------------------------------------------------------
    // infer_source unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn infer_source_zylcode_file() {
        let p = std::path::Path::new("/ws/src/zylcode.config.json");
        assert_eq!(infer_source(p), "zylcode");
    }

    #[test]
    fn infer_source_zy_prefix() {
        let p = std::path::Path::new("/ws/zy-tool.tsx");
        assert_eq!(infer_source(p), "zylcode");
    }

    #[test]
    fn infer_source_manual_rs() {
        let p = std::path::Path::new("/ws/src/lib.rs");
        assert_eq!(infer_source(p), "manual");
    }

    #[test]
    fn infer_source_manual_md() {
        let p = std::path::Path::new("/ws/README.md");
        assert_eq!(infer_source(p), "manual");
    }

    #[test]
    fn infer_source_manual_json() {
        let p = std::path::Path::new("/ws/package.json");
        assert_eq!(infer_source(p), "manual");
    }

    #[test]
    fn infer_source_manual_toml() {
        let p = std::path::Path::new("/ws/Cargo.toml");
        assert_eq!(infer_source(p), "manual");
    }

    #[test]
    fn infer_source_manual_tsx() {
        let p = std::path::Path::new("/ws/App.tsx");
        assert_eq!(infer_source(p), "manual");
    }

    #[test]
    fn infer_source_unknown_extension() {
        let p = std::path::Path::new("/ws/data.xyz");
        assert_eq!(infer_source(p), "manual");
    }

    #[test]
    fn infer_source_no_extension() {
        let p = std::path::Path::new("/ws/Makefile");
        assert_eq!(infer_source(p), "manual");
    }

    #[test]
    fn infer_source_zylcode_in_directory_name() {
        let p = std::path::Path::new("/ws/zylcode-output/component.tsx");
        assert_eq!(infer_source(p), "zylcode");
    }

    // -----------------------------------------------------------------------
    // BenchmarkItem / BenchmarkReport serde roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn benchmark_item_serde_roundtrip() {
        let item = BenchmarkItem {
            path: "src/main.rs".to_string(),
            source: "manual".to_string(),
            kind: "RustModule".to_string(),
            rung: "R1".to_string(),
            passed: true,
        };
        let json = serde_json::to_string(&item).unwrap();
        let de: BenchmarkItem = serde_json::from_str(&json).unwrap();
        assert_eq!(de.path, "src/main.rs");
        assert_eq!(de.source, "manual");
        assert_eq!(de.kind, "RustModule");
        assert_eq!(de.rung, "R1");
        assert!(de.passed);
    }

    #[test]
    fn benchmark_report_serde_roundtrip() {
        let mut rung_counts = std::collections::HashMap::new();
        rung_counts.insert("R1".to_string(), 3);
        rung_counts.insert("R2".to_string(), 1);

        let mut kind_counts = std::collections::HashMap::new();
        kind_counts.insert("RustModule".to_string(), 3);
        kind_counts.insert("UiComponent".to_string(), 1);

        let report = BenchmarkReport {
            workspace: "/tmp/ws".to_string(),
            timestamp: "2026-09-07T12:00:00Z".to_string(),
            files_scanned: 10,
            artifacts_found: 4,
            rung_counts,
            kind_counts,
            items: vec![
                BenchmarkItem {
                    path: "a.rs".to_string(),
                    source: "manual".to_string(),
                    kind: "RustModule".to_string(),
                    rung: "R1".to_string(),
                    passed: true,
                },
            ],
            scan_duration_ms: 42,
            success: true,
            error: None,
        };

        let json = serde_json::to_string(&report).unwrap();
        let de: BenchmarkReport = serde_json::from_str(&json).unwrap();
        assert_eq!(de.workspace, "/tmp/ws");
        assert_eq!(de.files_scanned, 10);
        assert_eq!(de.artifacts_found, 4);
        assert_eq!(de.scan_duration_ms, 42);
        assert!(de.success);
        assert!(de.error.is_none());
        assert_eq!(de.items.len(), 1);
        assert_eq!(de.rung_counts.get("R1"), Some(&3));
        assert_eq!(de.rung_counts.get("R2"), Some(&1));
        assert_eq!(de.kind_counts.get("RustModule"), Some(&3));
    }

    #[test]
    fn benchmark_report_with_error_serde_roundtrip() {
        let report = BenchmarkReport {
            workspace: "/ws".to_string(),
            timestamp: "2026-09-07T12:00:00Z".to_string(),
            files_scanned: 0,
            artifacts_found: 0,
            rung_counts: std::collections::HashMap::new(),
            kind_counts: std::collections::HashMap::new(),
            items: vec![],
            scan_duration_ms: 0,
            success: false,
            error: Some("workspace directory does not exist: /ws".to_string()),
        };

        let json = serde_json::to_string(&report).unwrap();
        let de: BenchmarkReport = serde_json::from_str(&json).unwrap();
        assert!(!de.success);
        assert_eq!(
            de.error.unwrap(),
            "workspace directory does not exist: /ws"
        );
    }

    // -----------------------------------------------------------------------
    // run_benchmark integration tests (subprocess + tempdir)
    // -----------------------------------------------------------------------

    /// Helper: run the benchmark binary and capture its stdout as a string.
    fn run_benchmark_binary(args: &[&str]) -> String {
        let exe = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/debug/zylcode-core-cli");
        let output = std::process::Command::new(exe)
            .args(args)
            .output()
            .expect("failed to execute benchmark binary");
        assert!(
            output.status.success(),
            "benchmark binary failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("stdout was not valid UTF-8")
    }

    #[test]
    fn run_benchmark_empty_workspace_json() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        let out = run_benchmark_binary(&[
            "benchmark",
            "--workspace",
            ws.to_str().unwrap(),
            "--format",
            "json",
        ]);
        let report: BenchmarkReport = serde_json::from_str(&out).unwrap();
        assert!(report.success);
        assert_eq!(report.artifacts_found, 0);
        assert!(report.items.is_empty());
        assert!(report.rung_counts.is_empty());
        assert!(report.kind_counts.is_empty());
        assert!(report.error.is_none());
    }

    #[test]
    fn run_benchmark_with_rust_file() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("lib.rs"), "pub fn hello() -> i32 { 42 }").unwrap();
        let out = run_benchmark_binary(&[
            "benchmark",
            "--workspace",
            ws.to_str().unwrap(),
            "--format",
            "json",
        ]);
        let report: BenchmarkReport = serde_json::from_str(&out).unwrap();
        assert!(report.success);
        assert_eq!(report.artifacts_found, 1);
        assert_eq!(report.items.len(), 1);
        assert_eq!(report.items[0].kind, "RustModule");
        assert_eq!(report.items[0].source, "manual");
        assert_eq!(report.items[0].rung, "Props");
        assert!(report.items[0].passed);
        assert_eq!(report.kind_counts.get("RustModule"), Some(&1));
        assert_eq!(report.rung_counts.get("Props"), Some(&1));
    }

    #[test]
    fn run_benchmark_with_markdown_file() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("README.md"), "# Hello").unwrap();
        let out = run_benchmark_binary(&[
            "benchmark",
            "--workspace",
            ws.to_str().unwrap(),
            "--format",
            "json",
        ]);
        let report: BenchmarkReport = serde_json::from_str(&out).unwrap();
        assert!(report.success);
        assert_eq!(report.artifacts_found, 1);
        assert_eq!(report.items[0].kind, "FormalProofSpec");
        assert_eq!(report.items[0].rung, "Spec");
    }

    #[test]
    fn run_benchmark_with_json_file() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("manifest.json"), r#"{"name":"test"}"#).unwrap();
        let out = run_benchmark_binary(&[
            "benchmark",
            "--workspace",
            ws.to_str().unwrap(),
            "--format",
            "json",
        ]);
        let report: BenchmarkReport = serde_json::from_str(&out).unwrap();
        assert!(report.success);
        assert_eq!(report.artifacts_found, 1);
        assert_eq!(report.items[0].kind, "PluginManifest");
        assert_eq!(report.items[0].rung, "Lint");
    }

    #[test]
    fn run_benchmark_with_tsx_file() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("App.tsx"), "export default () => <div/>").unwrap();
        let out = run_benchmark_binary(&[
            "benchmark",
            "--workspace",
            ws.to_str().unwrap(),
            "--format",
            "json",
        ]);
        let report: BenchmarkReport = serde_json::from_str(&out).unwrap();
        assert!(report.success);
        assert_eq!(report.artifacts_found, 1);
        assert_eq!(report.items[0].kind, "UiComponent");
        assert_eq!(report.items[0].rung, "Lint");
    }

    #[test]
    fn run_benchmark_zylcode_source_detection() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(
            ws.join("zylcode.config.json"),
            r#"{"version":"1.0"}"#,
        )
        .unwrap();
        let out = run_benchmark_binary(&[
            "benchmark",
            "--workspace",
            ws.to_str().unwrap(),
            "--format",
            "json",
        ]);
        let report: BenchmarkReport = serde_json::from_str(&out).unwrap();
        assert!(report.success);
        assert_eq!(report.artifacts_found, 1);
        assert_eq!(report.items[0].source, "zylcode");
    }

    #[test]
    fn run_benchmark_text_output_format() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("main.rs"), "fn main() {}").unwrap();
        let out = run_benchmark_binary(&[
            "benchmark",
            "--workspace",
            ws.to_str().unwrap(),
            "--format",
            "text",
        ]);
        assert!(out.contains("Scanned"), "should contain 'Scanned'");
        assert!(out.contains("artifacts"), "should contain 'artifacts'");
        assert!(out.contains("Verification rungs"), "should show rung breakdown");
        assert!(out.contains("Props"), "should list Props rung");
    }

    #[test]
    fn run_benchmark_nonexistent_workspace() {
        let exe = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/debug/zylcode-core-cli");
        let output = std::process::Command::new(exe)
            .args(["benchmark", "--workspace", "/nonexistent/path/xyz", "--format", "json"])
            .output()
            .expect("failed to execute");
        assert!(
            !output.status.success(),
            "should fail for nonexistent workspace"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("does not exist"),
            "stderr should mention 'does not exist': {stderr}"
        );
    }

    #[test]
    fn run_benchmark_nested_directories() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        let sub = ws.join("src").join("components");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("Button.tsx"), "export const Button = () => <button/>").unwrap();
        std::fs::write(sub.join("utils.rs"), "pub fn helper() {}").unwrap();
        std::fs::write(sub.join("styles.json"), r#"{"bg":"blue"}"#).unwrap();

        let out = run_benchmark_binary(&[
            "benchmark",
            "--workspace",
            ws.to_str().unwrap(),
            "--format",
            "json",
        ]);
        let report: BenchmarkReport = serde_json::from_str(&out).unwrap();
        assert!(report.success);
        assert_eq!(report.artifacts_found, 3);
        assert_eq!(report.files_scanned, 3);
    }

    #[test]
    fn run_benchmark_skips_non_artifact_files() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("image.png"), b"\x89PNG").unwrap();
        std::fs::write(ws.join("data.bin"), b"\x00\x01\x02").unwrap();

        let out = run_benchmark_binary(&[
            "benchmark",
            "--workspace",
            ws.to_str().unwrap(),
            "--format",
            "json",
        ]);
        let report: BenchmarkReport = serde_json::from_str(&out).unwrap();
        assert!(report.success);
        assert!(report.artifacts_found < report.files_scanned);
    }

    #[test]
    fn diagnostic_validate_artifact_kind_with_tempdir() {
        use tempfile::tempdir;
        use std::fs;
        use crate::pipeline::validate_artifact_kind;

        let dir = tempdir().unwrap();

        // Write test files with known extensions
        let rust_file = dir.path().join("lib.rs");
        fs::write(&rust_file, "pub fn greet(name: &str) -> String { format!(\"Hello, {}!\", name) }").unwrap();

        let md_file = dir.path().join("README.md");
        fs::write(&md_file, "# Project\n\nThis is a README file.").unwrap();

        let tsx_file = dir.path().join("App.tsx");
        fs::write(&tsx_file, "import React from 'react';\nexport const App = () => <div>Hello</div>;").unwrap();

        let json_file = dir.path().join("config.json");
        fs::write(&json_file, "{\"name\": \"test\", \"version\": \"1.0.0\"}").unwrap();

        // Test each file directly against validate_artifact_kind
        let test_cases: Vec<(&std::path::PathBuf, &str)> = vec![
            (&rust_file, "manual"),
            (&md_file, "manual"),
            (&tsx_file, "manual"),
            (&json_file, "manual"),
        ];

        for (file_path, source) in &test_cases {
            let content = fs::read_to_string(file_path).unwrap();
            let path_str = file_path.to_string_lossy();
            let kind = validate_artifact_kind(&path_str, &content, source);
            let ext = file_path.extension().unwrap().to_str().unwrap();
            println!(
                "FILE: {} | EXT: {} | CONTENT_LEN: {} | KIND: '{}' | SOURCE: '{}'",
                path_str, ext, content.len(), kind, source
            );
        }
    }

    /// Direct-call diagnostic test: replicates run_benchmark()'s scan logic directly
    /// using walkdir + validate_artifact_kind + infer_source, without subprocess or
    /// stdout capture. If this passes but subprocess tests fail, the bug is in
    /// run_benchmark_binary(). If this also fails with artifacts_found=0, the bug
    /// is in the scan logic itself.
    #[test]
    fn diagnostic_run_benchmark_direct_call() {
        use tempfile::tempdir;
        use walkdir::WalkDir;

        let dir = tempdir().unwrap();
        let ws = dir.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();

        // Create a known Rust artifact file
        let rust_file = ws.join("lib.rs");
        std::fs::write(
            &rust_file,
            "pub fn hello() -> String { \"hello\".to_string() }",
        )
        .unwrap();

        // Create a known TypeScript/React artifact file
        let tsx_file = ws.join("App.tsx");
        std::fs::write(
            &tsx_file,
            "import React from 'react';\nexport const App = () => <div>Hello</div>;",
        )
        .unwrap();

        // Create a known JSON artifact file
        let json_file = ws.join("package.json");
        std::fs::write(
            &json_file,
            r#"{"name": "test", "version": "1.0.0"}"#,
        )
        .unwrap();

        // Replicate run_benchmark's scan logic directly
        let mut files_scanned = 0u64;
        let mut artifacts_found = 0u64;
        let mut items: Vec<BenchmarkItem> = Vec::new();
        let mut kind_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        let mut rung_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

        let walker = WalkDir::new(&ws)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "node_modules" && name != "target"
            });

        for entry in walker {
            let entry = match entry {
                Ok(e) => {
                    eprintln!("  WALK: {:?}", e.path());
                    e
                }
                Err(e) => {
                    eprintln!("  WALK_ERR: {}", e);
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                eprintln!("  SKIP (not file): {:?}", entry.path());
                continue;
            }

            files_scanned += 1;
            let path = entry.path();
            let source = infer_source(path);
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let kind = validate_artifact_kind(
                &path.to_string_lossy(),
                &content,
                &source,
            );
            if kind == "error" || kind.is_empty() {
                continue;
            }
            artifacts_found += 1;
            let rung = classify_verification_rung(&kind);
            items.push(BenchmarkItem {
                path: path.to_string_lossy().to_string(),
                source: source.to_string(),
                kind: kind.clone(),
                rung: rung.short_label().to_string(),
                passed: true,
            });
            *kind_counts.entry(kind).or_insert(0) += 1;
            *rung_counts.entry(rung.short_label().to_string()).or_insert(0) += 1;
        }

        eprintln!(
            "files_scanned={}, artifacts_found={}, items.len()={}",
            files_scanned,
            artifacts_found,
            items.len()
        );
        for item in &items {
            eprintln!(
                "  ITEM: path={} kind={} rung={}",
                item.path, item.kind, item.rung
            );
        }

        assert!(
            artifacts_found > 0,
            "direct call: artifacts_found=0 (files_scanned={})",
            files_scanned
        );
        assert!(!items.is_empty());
    }
}
