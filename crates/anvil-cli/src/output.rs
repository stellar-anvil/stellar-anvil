use colored::Colorize;

pub fn step(msg: &str) {
    println!("  {} {}", "→".cyan(), msg);
}

pub fn ok(msg: &str) {
    println!("  {} {}", "✓".green().bold(), msg);
}

pub fn warn(msg: &str) {
    println!("  {} {}", "⚠".yellow().bold(), msg);
}

pub fn fail(msg: &str) {
    eprintln!("  {} {}", "✗".red().bold(), msg);
}

pub fn section(title: &str) {
    println!("\n{}", title.white().bold());
    println!("{}", "─".repeat(60).dimmed());
}

pub fn value(key: &str, val: &str) {
    println!("  {:<24} {}", key.dimmed(), val.cyan());
}
