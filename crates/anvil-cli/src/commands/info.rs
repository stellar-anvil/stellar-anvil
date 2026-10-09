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

pub async fn run(args: InfoArgs, _verbose: bool) -> Result<()> {
    output::section("Stellar Anvil Info");

    output::value("anvil version", env!("CARGO_PKG_VERSION"));

    // Check stellar-cli is available
    let stellar_ver = Command::new("stellar")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "not found".red().to_string());

    output::value("stellar-cli", &stellar_ver);

    // Check cargo
    let cargo_ver = Command::new("cargo")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "not found".red().to_string());

    output::value("cargo", &cargo_ver);

    // Check rustc
    let rustc_ver = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "not found".red().to_string());

    output::value("rustc", &rustc_ver);

    // Detect workspace root
    let workspace = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".into());
    output::value("workspace", &workspace);

    if args.versions {
        println!();
        output::section("Crate Versions");
        let crates = [
            ("anvil-core",     "0.1.0"),
            ("anvil-profiler", "0.1.0"),
            ("anvil-multi",    "0.1.0"),
            ("anvil-auth",     "0.1.0"),
            ("anvil-cli",      "0.1.0"),
        ];
        for (name, ver) in &crates {
            output::value(name, ver);
        }
    }

    println!();
    println!("  {}", "Documentation".cyan().bold());
    println!("  https://github.com/your-org/stellar-anvil");
    println!("  https://developers.stellar.org/docs/build/guides/testing");
    println!();

    Ok(())
}
