use soroban_sdk::{Address, Env};
use std::collections::HashMap;
use colored::Colorize;

/// Metadata stored for each contract in the registry.
#[derive(Debug, Clone)]
pub struct ContractEntry {
    /// Human-readable name
    pub name: String,
    /// On-chain address
    pub address: Address,
    /// Optional version tag for tracking upgrades
    pub version: Option<String>,
    /// Contracts this one depends on (by name)
    pub depends_on: Vec<String>,
}

/// A named registry of deployed contracts for a single test scenario.
///
/// Provides dependency-aware deployment ordering and address lookup by name.
///
/// # Example
/// ```ignore
/// let mut registry = ContractRegistry::new(&env);
/// registry.add("token",   token_address.clone(),   &[]);
/// registry.add("vault",   vault_address.clone(),   &["token"]);
/// registry.add("router",  router_address.clone(),  &["vault", "token"]);
///
/// let vault_addr = registry.get("vault").unwrap();
/// ```
pub struct ContractRegistry {
    env: Env,
    entries: HashMap<String, ContractEntry>,
    /// Insertion-ordered names so we can print in a stable order
    order: Vec<String>,
}

impl ContractRegistry {
    pub fn new(env: &Env) -> Self {
        Self {
            env: env.clone(),
            entries: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Register a contract under a name with optional dependencies.
    pub fn add(
        &mut self,
        name: impl Into<String>,
        address: Address,
        depends_on: &[&str],
    ) -> &ContractEntry {
        let name = name.into();
        let entry = ContractEntry {
            name: name.clone(),
            address,
            version: None,
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
        };
        self.entries.insert(name.clone(), entry);
        if !self.order.contains(&name) {
            self.order.push(name.clone());
        }
        self.entries.get(&name).unwrap()
    }

    /// Tag a registered contract with a version string.
    pub fn set_version(&mut self, name: &str, version: impl Into<String>) {
        if let Some(entry) = self.entries.get_mut(name) {
            entry.version = Some(version.into());
        }
    }

    /// Retrieve a contract address by name.
    pub fn get(&self, name: &str) -> Option<&Address> {
        self.entries.get(name).map(|e| &e.address)
    }

    /// Retrieve a contract entry by name.
    pub fn entry(&self, name: &str) -> Option<&ContractEntry> {
        self.entries.get(name)
    }

    /// Return all registered contracts in insertion order.
    pub fn all(&self) -> Vec<&ContractEntry> {
        self.order
            .iter()
            .filter_map(|n| self.entries.get(n))
            .collect()
    }

    /// Topological sort of contracts based on `depends_on`.
    /// Returns names in dependency-first order, or an error message if a cycle exists.
    pub fn deployment_order(&self) -> Result<Vec<&str>, String> {
        let mut visited = std::collections::HashSet::new();
        let mut stack = Vec::new();
        let mut result = Vec::new();

        fn visit<'a>(
            name: &'a str,
            entries: &'a HashMap<String, ContractEntry>,
            visited: &mut std::collections::HashSet<String>,
            stack: &mut Vec<String>,
            result: &mut Vec<&'a str>,
        ) -> Result<(), String> {
            if stack.contains(&name.to_string()) {
                return Err(format!("Circular dependency detected at '{}'", name));
            }
            if visited.contains(name) {
                return Ok(());
            }
            stack.push(name.to_string());
            if let Some(entry) = entries.get(name) {
                for dep in &entry.depends_on {
                    visit(dep, entries, visited, stack, result)?;
                }
            }
            stack.pop();
            visited.insert(name.to_string());
            result.push(name);
            Ok(())
        }

        // Collect names first to avoid borrow issues
        let names: Vec<String> = self.order.clone();
        for name in &names {
            visit(name, &self.entries, &mut visited, &mut stack, &mut result)?;
        }
        Ok(result)
    }

    /// Print the registry as a dependency tree.
    pub fn print_tree(&self) {
        println!("\n{}", "Contract Registry".cyan().bold());
        println!("{}", "─".repeat(50).dimmed());
        for entry in self.all() {
            let ver = entry
                .version
                .as_deref()
                .map(|v| format!(" ({})", v))
                .unwrap_or_default();
            let deps = if entry.depends_on.is_empty() {
                String::new()
            } else {
                format!(" ← [{}]", entry.depends_on.join(", ").dimmed())
            };
            println!(
                "  {} {}{}{}",
                "◆".cyan(),
                entry.name.white().bold(),
                ver.dimmed(),
                deps,
            );
        }
        println!();
    }
}
