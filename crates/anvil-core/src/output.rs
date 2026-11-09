//! Shared output helpers for Stellar Anvil library crates.
//!
//! ## Color control
//!
//! All crates in the workspace use the [`colored`] crate for terminal output.
//! Color is controlled via a **process-global** flag — calling [`configure_color`]
//! once (e.g. at the top of `main`) is sufficient to affect every crate.
//!
//! The flag is automatically honored when:
//! - You call `configure_color(true)` explicitly.
//! - The `NO_COLOR` environment variable is set (any value).
//! - `TERM=dumb` is set.
//!
//! When using the `stellar-anvil` CLI, color is configured automatically before
//! any output is produced. When using the library crates directly in tests you
//! can call [`configure_color`] yourself or simply let `colored` auto-detect.

/// Configure ANSI color output for all Stellar Anvil library crates.
///
/// Call this once at the start of your binary / test harness entry point.
///
/// ```no_run
/// anvil_core::output::configure_color(false); // force color on
/// anvil_core::output::configure_color(true);  // force color off
/// ```
pub fn configure_color(disable: bool) {
    let no_color_env = std::env::var("NO_COLOR").is_ok();
    let dumb_term    = std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false);

    if disable || no_color_env || dumb_term {
        colored::control::set_override(false);
    } else {
        // Let `colored` decide via its own TTY / platform detection.
        colored::control::unset_override();
    }
}

/// Format a `u64` with thousands separators: `23_847_291` → `"23,847,291"`.
pub fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}
