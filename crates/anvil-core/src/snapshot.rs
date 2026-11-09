use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::error::AnvilError;

/// Configuration for creating or loading a ledger snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    /// Path to the snapshot JSON file
    pub path: PathBuf,
    /// Optional ledger sequence number to snapshot at (None = latest)
    pub ledger_seq: Option<u32>,
    /// Contract addresses to include in the snapshot
    pub contracts: Vec<String>,
    /// Network passphrase — defaults to testnet
    pub network: String,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("snapshot.json"),
            ledger_seq: None,
            contracts: Vec::new(),
            network: "Test SDF Network ; September 2015".to_string(),
        }
    }
}

impl SnapshotConfig {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            ..Default::default()
        }
    }

    pub fn with_contract(mut self, address: impl Into<String>) -> Self {
        self.contracts.push(address.into());
        self
    }

    pub fn at_ledger(mut self, seq: u32) -> Self {
        self.ledger_seq = Some(seq);
        self
    }

    pub fn on_mainnet(mut self) -> Self {
        self.network = "Public Global Stellar Network ; September 2015".to_string();
        self
    }
}

/// In-memory representation of a ledger snapshot.
/// Wraps a JSON snapshot file produced by `stellar snapshot create`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerSnapshot {
    pub config: SnapshotConfig,
    /// Raw JSON content of the snapshot file
    pub raw: serde_json::Value,
}

impl LedgerSnapshot {
    /// Load a snapshot from a JSON file on disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, AnvilError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| AnvilError::SnapshotLoad(format!("{}: {}", path.display(), e)))?;
        let raw: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| AnvilError::SnapshotLoad(format!("invalid JSON: {}", e)))?;
        Ok(Self {
            config: SnapshotConfig::new(path.to_path_buf()),
            raw,
        })
    }

    /// Save the snapshot back to disk (useful after mutations in tests).
    pub fn save(&self) -> Result<(), AnvilError> {
        let content = serde_json::to_string_pretty(&self.raw)?;
        std::fs::write(&self.config.path, content)?;
        Ok(())
    }

    /// Return the path this snapshot is backed by.
    pub fn path(&self) -> &Path {
        &self.config.path
    }
}
