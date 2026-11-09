use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::process::Command;
use crate::output;

#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Show version info for all Anvil crates
    #[arg(long)]
    pub versions: bool,
}

/// Crate versions resolved at compile time from each crate's own CARGO_PKG_VERSION.
/// These are set via build.rs environment variables so they always stay in sync
/// with the workspace Cargo.toml — no manual string literals to drift.
///
/// Because anvil-cli depends on all sibling crates, we use the CLI crate's version
/// as the authoritative workspace version.  Individual crate versions are exposed
/// via their own `VERSION` constants (see each crate's lib.rs).
const WORKSPACE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn run(args: InfoArgs, _verbose: bool, quiet: bool) -> Result<()> {
    if quiet {
        return Ok(());
    }

    output::section("Stellar Anvil Info");

    output::value("anvil version", WORKSPACE_VERSION);

    // Check stellar-cli
    let stellar_ver = Command::new("stellar")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        })
        .unwrap_or_else(|| "not found".red().to_string());

    output::value("stellar-cli", &stellar_ver);

    // Check cargo
    let cargo_ver = Command::new("cargo")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        })
        .unwrap_or_else(|| "not found".red().to_string());

    output::value("cargo", &cargo_ver);

    // Check rustc
    let rustc_ver = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        })
        .unwrap_or_else(|| "not found".red().to_string());

    output::value("rustc", &rustc_ver);

    // Workspace root
    let workspace = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".into());
    output::value("workspace", &workspace);

    if args.versions {
        println!();
        output::section("Crate Versions");
        // All crates share the workspace version — read from the single
        // CARGO_PKG_VERSION constant so it can never drift.
        let crates = [
            "anvil-core",
            "anvil-profiler",
            "anvil-multi",
            "anvil-auth",
            "anvil-cli",
        ];
        for name in &crates {
            output::value(name, WORKSPACE_VERSION);
        }
    }

    println!();
    println!("  {}", "Documentation".cyan().bold());
    println!("  {}", "https://github.com/stellar-anvil/stellar-anvil".dimmed());
    println!("  {}", "https://developers.stellar.org/docs/build/guides/testing".dimmed());
    println!();

    Ok(())
}
