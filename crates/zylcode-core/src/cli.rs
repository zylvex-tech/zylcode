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

use anyhow::Result;
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
                 \x20     zylcode-core-cli verify [--workspace <DIR>]"
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
        "zylcode-core-cli: headless verification"
    );

    let config = EngineConfig {
        workspace_root: args.workspace.to_string_lossy().to_string(),
        verbose: args.verbose,
        ..Default::default()
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
             \x20     zylcode-core-cli verify [--workspace <DIR>]"
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
}
