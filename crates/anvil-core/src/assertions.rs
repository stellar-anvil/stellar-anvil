use colored::Colorize;

/// Rich assertion helpers for Stellar Anvil tests.
/// Produces human-readable failure messages rather than raw panics.
pub struct AnvilAssert;

impl AnvilAssert {
    /// Assert two values are equal. Panics with a colored diff on failure.
    pub fn eq<T: PartialEq + std::fmt::Debug>(label: &str, expected: T, actual: T) {
        if expected != actual {
            panic!(
                "\n{}\n  label:    {}\n  expected: {:?}\n  actual:   {:?}\n",
                "ASSERTION FAILED".red().bold(),
                label.yellow(),
                expected,
                actual,
            );
        }
        println!("{} {} = {:?}", "✓".green(), label, actual);
    }

    /// Assert a condition is true.
    pub fn is_true(label: &str, condition: bool) {
        if !condition {
            panic!(
                "\n{}\n  label: {}\n  expected condition to be true\n",
                "ASSERTION FAILED".red().bold(),
                label.yellow(),
            );
        }
        println!("{} {}", "✓".green(), label);
    }

    /// Assert a value is greater than a minimum.
    pub fn gt<T: PartialOrd + std::fmt::Debug>(label: &str, value: T, min: T) {
        if value <= min {
            panic!(
                "\n{}\n  label: {}\n  expected {:?} > {:?}\n",
                "ASSERTION FAILED".red().bold(),
                label.yellow(),
                value,
                min,
            );
        }
        println!("{} {} = {:?} (> {:?})", "✓".green(), label, value, min);
    }

    /// Assert a value is within an inclusive range [lo, hi].
    pub fn in_range<T: PartialOrd + std::fmt::Debug>(label: &str, value: T, lo: T, hi: T) {
        if value < lo || value > hi {
            panic!(
                "\n{}\n  label: {}\n  expected {:?} in [{:?}, {:?}]\n",
                "ASSERTION FAILED".red().bold(),
                label.yellow(),
                value,
                lo,
                hi,
            );
        }
        println!("{} {} = {:?} (in range)", "✓".green(), label, value);
    }

    /// Assert that a Result is Ok and return the inner value.
    pub fn ok<T, E: std::fmt::Debug>(label: &str, result: Result<T, E>) -> T {
        match result {
            Ok(v) => {
                println!("{} {} is Ok", "✓".green(), label);
                v
            }
            Err(e) => {
                panic!(
                    "\n{}\n  label: {}\n  expected Ok, got Err({:?})\n",
                    "ASSERTION FAILED".red().bold(),
                    label.yellow(),
                    e,
                );
            }
        }
    }

    /// Assert that a Result is Err.
    pub fn err<T: std::fmt::Debug, E: std::fmt::Debug>(label: &str, result: Result<T, E>) {
        match result {
            Err(_) => println!("{} {} correctly returned an error", "✓".green(), label),
            Ok(v) => {
                panic!(
                    "\n{}\n  label: {}\n  expected Err, got Ok({:?})\n",
                    "ASSERTION FAILED".red().bold(),
                    label.yellow(),
                    v,
                );
            }
        }
    }
}
