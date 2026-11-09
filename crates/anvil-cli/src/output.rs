use colored::Colorize;

// ─────────────────────────────────────────────────────────────────────────────
// Color control
// ─────────────────────────────────────────────────────────────────────────────

/// Call once at startup. Disables all ANSI color if:
///  - `--no-color` was passed (caller passes `true`)
///  - `NO_COLOR` env var is set (any value)
///  - `TERM=dumb`
///  - stdout is not a TTY (piped/redirected)
pub fn configure_color(no_color_flag: bool) {
    let no_color_env = std::env::var("NO_COLOR").is_ok();
    let dumb_term = std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false);
    // atty not available as a dep — use colored's own tty detection via
    // the override API instead (colored v2 checks atty internally when no
    // override is set, but we set an explicit override to be safe).
    if no_color_flag || no_color_env || dumb_term {
        colored::control::set_override(false);
    } else {
        // Let `colored` decide via its own TTY detection.
        colored::control::unset_override();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Number formatting
// ─────────────────────────────────────────────────────────────────────────────

/// Format a u64 with thousands separators, e.g. 23_847_291 → "23,847,291".
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

/// Format a delta percentage with sign and thousands-separator, e.g. "+12,345.6%".
pub fn fmt_delta(pct: f64) -> String {
    format!("{:>+8.1}%", pct)
}

// ─────────────────────────────────────────────────────────────────────────────
// Section / message helpers
// ─────────────────────────────────────────────────────────────────────────────

const LINE_WIDTH: usize = 78;

/// Print a Foundry-style section header:
///   ── Title ─────────────────────────────────
pub fn section(title: &str) {
    let prefix = format!("── {} ", title);
    let fill = LINE_WIDTH.saturating_sub(prefix.len());
    let line = format!("{}{}", prefix, "─".repeat(fill));
    println!("\n{}", line.cyan().bold());
}

/// Print a sub-section divider (thinner rule, no label).
pub fn divider() {
    println!("{}", "─".repeat(LINE_WIDTH).dimmed());
}

/// Print a heavy divider (═ rule, no label) — used at start/end of report blocks.
pub fn heavy_divider() {
    println!("{}", "═".repeat(LINE_WIDTH).dimmed());
}

pub fn step(msg: &str) {
    println!("  {} {}", "→".cyan(), msg);
}

pub fn ok(msg: &str) {
    println!("  {} {}", "✓".green().bold(), msg);
}

pub fn warn(msg: &str) {
    println!("  {} {}", "⚠".yellow().bold(), msg);
}

/// Print an error line to stderr. Prefer `anyhow::bail!` for fatal errors that
/// should terminate the command; use this for non-fatal per-item failures within
/// a larger run.
pub fn fail(msg: &str) {
    eprintln!("  {} {}", "✗".red().bold(), msg);
}

pub fn value(key: &str, val: &str) {
    println!("  {:<24} {}", key.dimmed(), val.cyan());
}

/// Print a color legend line — call at the bottom of any colored table.
pub fn legend(items: &[(&str, &str)]) {
    let parts: Vec<String> = items
        .iter()
        .map(|(color, label)| format!("{} {}", color, label))
        .collect();
    println!("\n  {}", parts.join("   ").dimmed());
}

// ─────────────────────────────────────────────────────────────────────────────
// Delta coloring (shared between profile.rs and compare.rs)
// ─────────────────────────────────────────────────────────────────────────────

pub fn color_delta(pct: f64) -> String {
    let s = fmt_delta(pct);
    if pct > 10.0 {
        s.red().bold().to_string()
    } else if pct > 2.0 {
        s.yellow().to_string()
    } else if pct < -2.0 {
        s.green().to_string()
    } else {
        s.dimmed().to_string()
    }
}
