use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cinematography_ir::{
    analyze_continuity, load_project, render_summary, save_project_json, validate_project,
    CineProject, Severity, ValidationReport,
};
use schemars::schema_for;

#[derive(Debug, Parser)]
#[command(name = "cine-ir")]
#[command(about = "Validate, inspect, and exchange Cinematography IR documents")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run structural, numeric, reference, timeline, and continuity checks.
    Validate {
        input: PathBuf,
        /// Emit machine-readable diagnostics.
        #[arg(long)]
        json: bool,
        /// Return a non-zero status when warnings are present.
        #[arg(long)]
        deny_warnings: bool,
    },
    /// Run only cross-shot continuity analysis.
    Analyze {
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Print a compact human-readable shot plan.
    Inspect { input: PathBuf },
    /// Convert YAML or JSON into canonical pretty-printed JSON.
    Normalize {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate JSON Schema from the current Rust model.
    Schema {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Validate {
            input,
            json,
            deny_warnings,
        } => {
            let project = load_project(&input)?;
            let report = validate_project(&project);
            print_report(&report, json)?;
            if report.has_errors() || (deny_warnings && report.has_warnings()) {
                std::process::exit(2);
            }
        }
        Command::Analyze { input, json } => {
            let project = load_project(&input)?;
            let mut report = ValidationReport::default();
            for scene in &project.scenes {
                report.extend(analyze_continuity(scene).diagnostics);
            }
            print_report(&report, json)?;
            if report.has_errors() {
                std::process::exit(2);
            }
        }
        Command::Inspect { input } => {
            let project = load_project(&input)?;
            print!("{}", render_summary(&project));
        }
        Command::Normalize { input, output } => {
            let project = load_project(&input)?;
            match output {
                Some(path) => save_project_json(&project, path)?,
                None => println!("{}", serde_json::to_string_pretty(&project)?),
            }
        }
        Command::Schema { output } => {
            let schema = schema_for!(CineProject);
            let text = serde_json::to_string_pretty(&schema)?;
            match output {
                Some(path) => fs::write(&path, format!("{text}\n"))
                    .with_context(|| format!("failed to write {}", path.display()))?,
                None => println!("{text}"),
            }
        }
    }

    Ok(())
}

fn print_report(report: &ValidationReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }

    if report.diagnostics.is_empty() {
        println!("OK: no diagnostics");
        return Ok(());
    }

    for diagnostic in &report.diagnostics {
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        println!(
            "{severity}[{}] {}: {}",
            diagnostic.code, diagnostic.path, diagnostic.message
        );
        if let Some(hint) = &diagnostic.hint {
            println!("  hint: {hint}");
        }
    }

    println!(
        "\n{} error(s), {} warning(s)",
        report.error_count(),
        report.warning_count()
    );
    Ok(())
}
