use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;
use std::collections::HashMap;
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

pub async fn run(args: SnapshotArgs, verbose: bool, quiet: bool) -> Result<()> {
    match args.action {
        SnapshotAction::Create(a)  => create(a, verbose, quiet).await,
        SnapshotAction::Inspect(a) => inspect(a, verbose, quiet).await,
        SnapshotAction::Diff(a)    => diff(a, quiet).await,
    }
}

async fn create(args: SnapshotCreateArgs, verbose: bool, quiet: bool) -> Result<()> {
    if !quiet {
        output::section("Creating Ledger Snapshot");
        for address in &args.address {
            output::value("contract", address);
        }
        output::value("network", &args.network);
        output::value("output", &args.out.display().to_string());
        output::value("ledger seq", args.ledger.as_ref().map(|s| s.to_string()).as_deref().unwrap_or("latest"));
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

    if !quiet {
        output::step("Running stellar snapshot create…");
    }
    let result = cmd.output()?;

    if result.status.success() {
        if !quiet {
            output::ok(&format!("Snapshot saved to {}", args.out.display()));

            if args.out.exists() {
                let content = std::fs::read_to_string(&args.out)?;
                let json: serde_json::Value = serde_json::from_str(&content)?;
                if let Some(entries) = json.get("entries").and_then(|e| e.as_array()) {
                    output::value("ledger entries", &entries.len().to_string());
                }
            }
        }
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        anyhow::bail!("stellar snapshot create failed:\n{}", stderr);
    }

    Ok(())
}

async fn inspect(args: SnapshotInspectArgs, _verbose: bool, quiet: bool) -> Result<()> {
    if !quiet {
        output::section(&format!("Inspecting Snapshot: {}", args.path.display()));
    }

    let content = std::fs::read_to_string(&args.path)
        .map_err(|e| anyhow::anyhow!("cannot read '{}': {}", args.path.display(), e))?;

    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("invalid JSON in '{}': {}", args.path.display(), e))?;

    if args.raw {
        println!("{}", serde_json::to_string_pretty(&json)?);
        return Ok(());
    }

    if !quiet {
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
            output::divider();

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
    }

    Ok(())
}

async fn diff(args: SnapshotDiffArgs, quiet: bool) -> Result<()> {
    if !quiet {
        output::section("Snapshot Diff");
        output::value("A", &args.a.display().to_string());
        output::value("B", &args.b.display().to_string());
    }

    let a_content = std::fs::read_to_string(&args.a)
        .map_err(|e| anyhow::anyhow!("cannot read '{}': {}", args.a.display(), e))?;
    let b_content = std::fs::read_to_string(&args.b)
        .map_err(|e| anyhow::anyhow!("cannot read '{}': {}", args.b.display(), e))?;

    let a: serde_json::Value = serde_json::from_str(&a_content)
        .map_err(|e| anyhow::anyhow!("invalid JSON in snapshot A: {}", e))?;
    let b: serde_json::Value = serde_json::from_str(&b_content)
        .map_err(|e| anyhow::anyhow!("invalid JSON in snapshot B: {}", e))?;

    let a_ledger = a.get("latestLedger").and_then(|v| v.as_u64()).unwrap_or(0);
    let b_ledger = b.get("latestLedger").and_then(|v| v.as_u64()).unwrap_or(0);

    if !quiet {
        output::value("A ledger", &a_ledger.to_string());
        output::value("B ledger", &b_ledger.to_string());
    }

    // ── Correct diff: key entries by a stable canonical key field,
    //    not by raw JSON string serialization (which is non-deterministic
    //    for object key ordering and would produce phantom diffs).
    let a_map = build_entry_map(a.get("entries"));
    let b_map = build_entry_map(b.get("entries"));

    let mut added:   Vec<&String> = b_map.keys().filter(|k| !a_map.contains_key(*k)).collect();
    let mut removed: Vec<&String> = a_map.keys().filter(|k| !b_map.contains_key(*k)).collect();
    let mut changed: Vec<&String> = a_map.keys()
        .filter(|k| b_map.contains_key(*k) && a_map[*k] != b_map[*k])
        .collect();

    added.sort();
    removed.sort();
    changed.sort();

    if !quiet {
        println!();
        output::value("entries in A", &a_map.len().to_string());
        output::value("entries in B", &b_map.len().to_string());
        output::value("added in B",   &added.len().to_string());
        output::value("removed in B", &removed.len().to_string());
        output::value("changed",      &changed.len().to_string());

        if !added.is_empty() {
            println!("\n  {} New entries in B:", "+".green().bold());
            for k in added.iter().take(10) {
                let preview = truncate(k, 72);
                println!("    {} {}", "+".green(), preview);
            }
            if added.len() > 10 {
                println!("    {} … and {} more", "".dimmed(), added.len() - 10);
            }
        }

        if !removed.is_empty() {
            println!("\n  {} Entries removed in B:", "-".red().bold());
            for k in removed.iter().take(10) {
                let preview = truncate(k, 72);
                println!("    {} {}", "-".red(), preview);
            }
            if removed.len() > 10 {
                println!("    {} … and {} more", "".dimmed(), removed.len() - 10);
            }
        }

        if !changed.is_empty() {
            println!("\n  {} Entries changed:", "~".yellow().bold());
            for k in changed.iter().take(10) {
                let preview = truncate(k, 72);
                println!("    {} {}", "~".yellow(), preview);
            }
            if changed.len() > 10 {
                println!("    {} … and {} more", "".dimmed(), changed.len() - 10);
            }
        }

        if added.is_empty() && removed.is_empty() && changed.is_empty() {
            output::ok("Snapshots are identical");
        }
    }

    Ok(())
}

/// Build a map from a stable entry key → canonical JSON string of the full entry.
///
/// We use the `"key"` field of each entry as the map key.  This avoids the
/// non-deterministic JSON serialization problem that caused phantom diffs when
/// comparing HashSets of raw `serde_json::Value::to_string()` output.
fn build_entry_map(entries: Option<&serde_json::Value>) -> HashMap<String, String> {
    let Some(arr) = entries.and_then(|e| e.as_array()) else {
        return HashMap::new();
    };
    arr.iter()
        .filter_map(|entry| {
            // Use the "key" field as the stable map key.
            // Fall back to a stable JSON serialization of the whole entry only
            // if there is no "key" field.
            let key = entry
                .get("key")
                .map(|k| k.to_string())
                .unwrap_or_else(|| {
                    // Sort keys deterministically by converting to BTreeMap via
                    // a round-trip through serde.
                    canonical_json(entry)
                });
            // The value we store is a canonical serialization of the full entry.
            let value = canonical_json(entry);
            Some((key, value))
        })
        .collect()
}

/// Produce a key-sorted JSON string so that object-key ordering differences
/// don't create phantom diffs.
fn canonical_json(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(map) => {
            let sorted: std::collections::BTreeMap<_, _> = map.iter().collect();
            let parts: Vec<String> = sorted
                .iter()
                .map(|(k, val)| format!("\"{}\":{}", k, canonical_json(val)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        serde_json::Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", parts.join(","))
        }
        other => other.to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max])
    } else {
        s.to_string()
    }
}
