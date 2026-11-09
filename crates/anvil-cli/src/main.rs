//! # anvil-cli
//!
//! The `stellar-anvil` command-line tool. Part of the
//! [Stellar Anvil](https://github.com/stellar-anvil/stellar-anvil) toolkit.
//!
//! Install:
//! ```bash
//! cargo install --git https://github.com/stellar-anvil/stellar-anvil anvil-cli
//! ```
//!
//! See `stellar-anvil --help` for full usage.

mod commands;
mod output;

use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "stellar-anvil",
    version = env!("CARGO_PKG_VERSION"),
    about = "Foundry-inspired testing and simulation framework for Soroban smart contracts",
    long_about = None,
    arg_required_else_help = true,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output across all subcommands
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress all non-error output (overrides --verbose)
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Disable ANSI color output (also honored via NO_COLOR env var or TERM=dumb)
    #[arg(long, global = true)]
    no_color: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run Soroban contract tests with rich output
    Test(commands::test::TestArgs),

    /// Capture or load a ledger snapshot from testnet/mainnet
    Snapshot(commands::snapshot::SnapshotArgs),

    /// Profile CPU and memory usage of contract invocations
    Profile(commands::profile::ProfileArgs),

    /// Compare two profile reports and detect regressions
    Compare(commands::compare::CompareArgs),

    /// Print info about the current workspace
    Info(commands::info::InfoArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Configure color before any output is produced.
    output::configure_color(cli.no_color);

    if !cli.quiet {
        print_banner();
    }

    let result = match cli.command {
        Commands::Test(args)     => commands::test::run(args, cli.verbose, cli.quiet).await,
        Commands::Snapshot(args) => commands::snapshot::run(args, cli.verbose, cli.quiet).await,
        Commands::Profile(args)  => commands::profile::run(args, cli.verbose, cli.quiet).await,
        Commands::Compare(args)  => commands::compare::run(args, cli.verbose, cli.quiet).await,
        Commands::Info(args)     => commands::info::run(args, cli.verbose, cli.quiet).await,
    };

    if let Err(e) = result {
        eprintln!("\n{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn print_banner() {
    println!(
        "\n{}  {}\n",
        "⚡ stellar-anvil".cyan().bold(),
        env!("CARGO_PKG_VERSION").dimmed(),
    );
}
