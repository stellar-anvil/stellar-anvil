use colored::Colorize;
use serde::{Deserialize, Serialize};

/// A single node in a cross-contract call trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallNode {
    /// Name of the contract being called
    pub contract: String,
    /// Name of the function being called
    pub function: String,
    /// Caller contract/account name
    pub caller: String,
    /// Whether this call succeeded
    pub succeeded: bool,
    /// CPU instructions used for this call (captured from budget)
    pub cpu_instructions: u64,
    /// Sub-calls made from within this call
    pub sub_calls: Vec<CallNode>,
    /// Optional return value description
    pub return_value: Option<String>,
}

impl CallNode {
    pub fn new(
        caller: impl Into<String>,
        contract: impl Into<String>,
        function: impl Into<String>,
    ) -> Self {
        Self {
            caller: caller.into(),
            contract: contract.into(),
            function: function.into(),
            succeeded: true,
            cpu_instructions: 0,
            sub_calls: Vec::new(),
            return_value: None,
        }
    }

    pub fn with_cpu(mut self, cpu: u64) -> Self {
        self.cpu_instructions = cpu;
        self
    }

    pub fn with_return(mut self, rv: impl Into<String>) -> Self {
        self.return_value = Some(rv.into());
        self
    }

    pub fn failed(mut self) -> Self {
        self.succeeded = false;
        self
    }

    pub fn add_sub_call(&mut self, node: CallNode) {
        self.sub_calls.push(node);
    }

    fn print_indent(&self, depth: usize) {
        let indent = "  ".repeat(depth);
        let status = if self.succeeded {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        };
        let rv = self
            .return_value
            .as_deref()
            .map(|v| format!(" → {}", v.cyan()))
            .unwrap_or_default();
        println!(
            "{}{}  {}.{}(){}  [cpu: {}]",
            indent,
            status,
            self.contract.yellow().bold(),
            self.function.white(),
            rv,
            self.cpu_instructions.to_string().dimmed(),
        );
        for sub in &self.sub_calls {
            sub.print_indent(depth + 1);
        }
    }
}

/// A full call trace for one top-level transaction in a multi-contract scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallTrace {
    pub label: String,
    pub root: CallNode,
}

impl CallTrace {
    pub fn new(label: impl Into<String>, root: CallNode) -> Self {
        Self {
            label: label.into(),
            root,
        }
    }

    /// Print the trace as a tree.
    pub fn print(&self) {
        println!(
            "\n{} {}",
            "Call Trace:".cyan().bold(),
            self.label.white().bold()
        );
        println!("{}", "─".repeat(60).dimmed());
        self.root.print_indent(0);
        println!();
    }

    /// Total CPU across this trace (root + all sub-calls, recursively).
    pub fn total_cpu(&self) -> u64 {
        fn sum(node: &CallNode) -> u64 {
            node.cpu_instructions + node.sub_calls.iter().map(sum).sum::<u64>()
        }
        sum(&self.root)
    }

    /// Export to JSON.
    pub fn export_json(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}
