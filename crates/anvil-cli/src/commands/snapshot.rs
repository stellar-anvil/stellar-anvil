use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use std::process::Command;
use crate::output;

#[derive(Args, Debug)]
pub struct SnapshotArgs {
    #[command(subcommand)]
    pub action: SnapshotAction,
}

#[derive(Subcommand, Debug)]
pub enum SnapshotAction {
    /// Create a ledger snapshot from testnet or mainnet
    Create(SnapshotCreateArgs),
    /// Inspect the contents of a snapshot file
    Inspect(SnapshotInspectArgs),
    /// Diff two snapshot files
    Diff(SnapshotDiffArgs),
}

#[derive(Args, Debug)]
pub struct SnapshotCreateArgs {
    /// Contract address(es) to snapshot (can be repeated)
    #[arg(short, long, required = true)]
    pub address: Vec<String>,

    /// Output file path
    #[arg(short, long, default_value = "snapshot.json")]
    pub out: PathBuf,

    /// Ledger sequence to snapshot at (default: latest)
    #[arg(long)]
    pub ledger: Option<u32>,

    /// Network: testnet (default) or mainnet
    #[arg(long, default_value = "testnet")]
    pub network: String,

    /// RPC URL override
    #[arg(long)]
    pub rpc_url: Option<String>,
}

#[derive(Args, Debug)]
pub struct SnapshotInspectArgs {
    /// Path to snapshot JSON file
    pub path: PathBuf,

    /// Show raw JSON (default: summarized)
    #[arg(long)]
    pub raw: bool,
}

#[derive(Args, Debug)]
pub struct SnapshotDiffArgs {
    /// First snapshot file
    pub a: PathBuf,
    /// Second snapshot file
    pub b: PathBuf,
}

pub async fn run(args: SnapshotArgs, verbose: bool) -> Result<()> {
    match args.action {
        SnapshotAction::Create(a)  => create(a, verbose).await,
        SnapshotAction::Inspect(a) => inspect(a, verbose).await,
        SnapshotAction::Diff(a)    => diff(a, verbose).await,
    }
}

async fn create(args: SnapshotCreateArgs, verbose: bool) -> Result<()> {
    output::section("Creating Ledger Snapshot");

    for address in &args.address {
        output::value("contract", address);
    }
    output::value("network", &args.network);
    output::value("output", &args.out.display().to_string());

    if let Some(seq) = args.ledger {
        output::value("ledger seq", &seq.to_string());
    } else {
        output::value("ledger seq", "latest");
    }

    let mut cmd = Command::new("stellar");
    cmd.arg("snapshot").arg("create");

    for addr in &args.address {
        cmd.args(["--address", addr]);
    }

    cmd.args(["--output", "json"]);
    cmd.args(["--out", &args.out.display().to_string()]);

    match args.network.as_str() {
        "mainnet" => { cmd.args(["--network", "mainnet"]); }
        _         => { cmd.args(["--network", "testnet"]); }
    }

    if let Some(rpc) = &args.rpc_url {
        cmd.args(["--rpc-url", rpc]);
    }

    if let Some(seq) = args.ledger {
        cmd.args(["--ledger", &seq.to_string()]);
    }

    if verbose {
        output::value("command", &format!("{:?}", cmd));
    }

    output::step("Running stellar snapshot create...");
    let result = cmd.output()?;

    if result.status.success() {
        output::ok(&format!("Snapshot saved to {}", args.out.display()));

        if args.out.exists() {
            let content = std::fs::read_to_string(&args.out)?;
            let json: serde_json::Value = serde_json::from_str(&content)?;
            if let Some(entries) = json.get("entries").and_then(|e| e.as_array()) {
                output::value("ledger entries", &entries.len().to_string());
            }
        }
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        anyhow::bail!("stellar snapshot create failed:\n{}", stderr);
    }

    Ok(())
}

async fn inspect(args: SnapshotInspectArgs, _verbose: bool) -> Result<()> {
    output::section(&format!("Inspecting Snapshot: {}", args.path.display()));

    let content = std::fs::read_to_string(&args.path)
        .map_err(|e| anyhow::anyhow!("cannot read {}: {}", args.path.display(), e))?;

    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("invalid JSON: {}", e))?;

    if args.raw {
        println!("{}", serde_json::to_string_pretty(&json)?);
        return Ok(());
    }

    if let Some(ledger) = json.get("latestLedger") {
        output::value("latest ledger", &ledger.to_string());
    }
    if let Some(ts) = json.get("latestLedgerCloseTime") {
        output::value("close time", &ts.to_string());
    }
    if let Some(entries) = json.get("entries").and_then(|e| e.as_array()) {
        output::value("total entries", &entries.len().to_string());

        println!();
        println!("  {:<6} {:<20} {}", "#".dimmed(), "type".dimmed(), "key (truncated)".dimmed());
        println!("  {}", "─".repeat(60).dimmed());

        for (i, entry) in entries.iter().enumerate().take(20) {
            let entry_type = entry
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");
            let key = entry
                .get("key")
                .map(|k| {
                    let s = k.to_string();
                    if s.len() > 40 { format!("{}…", &s[..40]) } else { s }
                })
                .unwrap_or_else(|| "—".to_string());
            println!("  {:<6} {:<20} {}", i + 1, entry_type.cyan(), key.dimmed());
        }

        if entries.len() > 20 {
            println!("  {} … and {} more entries", "".dimmed(), entries.len() - 20);
        }
    }

    Ok(())
}

async fn diff(args: SnapshotDiffArgs, _verbose: bool) -> Result<()> {
    output::section("Snapshot Diff");
    output::value("A", &args.a.display().to_string());
    output::value("B", &args.b.display().to_string());

    let a_content = std::fs::read_to_string(&args.a)?;
    let b_content = std::fs::read_to_string(&args.b)?;

    let a: serde_json::Value = serde_json::from_str(&a_content)?;
    let b: serde_json::Value = serde_json::from_str(&b_content)?;

    let a_ledger = a.get("latestLedger").and_then(|v| v.as_u64()).unwrap_or(0);
    let b_ledger = b.get("latestLedger").and_then(|v| v.as_u64()).unwrap_or(0);

    output::value("A ledger", &a_ledger.to_string());
    output::value("B ledger", &b_ledger.to_string());

    let a_entries: Vec<String> = a
        .get("entries")
        .and_then(|e| e.as_array())
        .map(|arr| arr.iter().map(|v| v.to_string()).collect())
        .unwrap_or_default();

    let b_entries: Vec<String> = b
        .get("entries")
        .and_then(|e| e.as_array())
        .map(|arr| arr.iter().map(|v| v.to_string()).collect())
        .unwrap_or_default();

    let a_set: std::collections::HashSet<&String> = a_entries.iter().collect();
    let b_set: std::collections::HashSet<&String> = b_entries.iter().collect();

    let added:   Vec<&&String> = b_set.difference(&a_set).collect();
    let removed: Vec<&&String> = a_set.difference(&b_set).collect();

    println!();
    output::value("entries in A", &a_entries.len().to_string());
    output::value("entries in B", &b_entries.len().to_string());
    output::value("added in B",   &added.len().to_string());
    output::value("removed in B", &removed.len().to_string());

    if !added.is_empty() {
        println!("\n  {} New entries in B:", "+".green().bold());
        for e in added.iter().take(10) {
            let s = e.to_string();
            let preview = if s.len() > 72 { format!("{}…", &s[..72]) } else { s };
            println!("    {} {}", "+".green(), preview);
        }
    }

    if !removed.is_empty() {
        println!("\n  {} Entries removed from A:", "-".red().bold());
        for e in removed.iter().take(10) {
            let s = e.to_string();
            let preview = if s.len() > 72 { format!("{}…", &s[..72]) } else { s };
            println!("    {} {}", "-".red(), preview);
        }
    }

    Ok(())
}
