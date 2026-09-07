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
//! zylcode-core-cli <PROMPT> [--workspace <DIR>]
//! ```

use std::env;
use std::path::PathBuf;

use anyhow::Result;
use tracing::info;

use crate::{EngineConfig, Intent, ZylCodeEngine};

// ---------------------------------------------------------------------------
// Argument parsing (zero external deps — keeps the core crate lean)
// ---------------------------------------------------------------------------

/// Parsed CLI arguments.
#[derive(Debug, Clone)]
pub struct CliArgs {
    /// The natural-language prompt to process.
    pub prompt: String,
    /// Workspace root directory (defaults to cwd).
    pub workspace: PathBuf,
    /// Whether verbose tracing is enabled.
    pub verbose: bool,
}

/// Errors that can occur during argument parsing.
#[derive(Debug, Clone)]
pub enum ParseError {
    /// No arguments were provided.
    MissingPrompt,
    /// The `--workspace` flag was provided without a value.
    MissingWorkspaceValue,
    /// An unknown flag was encountered.
    UnknownFlag(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::MissingPrompt => write!(f, "usage: zylcode-core-cli <PROMPT> [--workspace <DIR>] [--verbose]"),
            ParseError::MissingWorkspaceValue => write!(f, "--workspace requires a directory path"),
            ParseError::UnknownFlag(flag) => write!(f, "unknown flag: {flag}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse CLI arguments from an iterator of strings.
///
/// Accepts:
/// - Positional arguments (concatenated into the prompt)
/// - `--workspace <DIR>` to override the working directory
/// - `--verbose` to enable tracing
pub fn parse_args<I, S>(args: I) -> Result<CliArgs, ParseError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut prompt_parts: Vec<String> = Vec::new();
    let mut workspace: Option<PathBuf> = None;
    let mut verbose = false;
    let mut args_vec: Vec<String> = args.into_iter().map(Into::into).collect();

    // Skip argv[0] (program name) if present — callers usually already
    // strip it, but be defensive.
    if let Some(first) = args_vec.first() {
        if !first.starts_with('-') && (first.contains('/') || first.contains('\\')) {
            args_vec.remove(0);
        }
    }

    let mut iter = args_vec.into_iter();
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
                // Everything after `--` is part of the prompt.
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

    Ok(CliArgs {
        prompt,
        workspace,
        verbose,
    })
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

/// Run the CLI: parse args, build engine, process intent, print result.
pub async fn run() -> Result<()> {
    let args = parse_args(env::args().skip(1))?;

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_simple_prompt() {
        let args = parse_args(["hello", "world"]).unwrap();
        assert_eq!(args.prompt, "hello world");
        assert!(!args.verbose);
    }

    #[test]
    fn parse_args_with_workspace() {
        let args = parse_args(["--workspace", "/tmp/project", "do", "something"]).unwrap();
        assert_eq!(args.prompt, "do something");
        assert_eq!(args.workspace, PathBuf::from("/tmp/project"));
    }

    #[test]
    fn parse_args_verbose_flag() {
        let args = parse_args(["--verbose", "test"]).unwrap();
        assert!(args.verbose);
        assert_eq!(args.prompt, "test");
    }

    #[test]
    fn parse_args_double_dash_terminates_flags() {
        let args = parse_args(["--", "--not-a-flag"]).unwrap();
        assert_eq!(args.prompt, "--not-a-flag");
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
        assert_eq!(ParseError::MissingPrompt.to_string(), "usage: zylcode-core-cli <PROMPT> [--workspace <DIR>] [--verbose]");
        assert_eq!(ParseError::MissingWorkspaceValue.to_string(), "--workspace requires a directory path");
        assert_eq!(
            ParseError::UnknownFlag("--foo".into()).to_string(),
            "unknown flag: --foo"
        );
    }

    #[tokio::test]
    async fn cli_end_to_end_prompt_in_diff_out() {
        let args = CliArgs {
            prompt: "add a README file".to_string(),
            workspace: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            verbose: false,
        };

        let config = EngineConfig {
            workspace_root: args.workspace.to_string_lossy().to_string(),
            verbose: false,
            ..Default::default()
        };

        let engine = ZylCodeEngine::new(config);
        let intent = Intent {
            prompt: args.prompt,
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
