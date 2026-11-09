use std::fmt;

/// All errors that can occur within Stellar Anvil.
#[derive(Debug)]
pub enum AnvilError {
    /// Snapshot file could not be read or parsed
    SnapshotLoad(String),
    /// Snapshot file could not be written
    SnapshotWrite(String),
    /// A required account was not found in the sandbox
    AccountNotFound(String),
    /// A contract address could not be resolved by alias
    ContractNotFound(String),
    /// Ledger time manipulation failed
    TimeTravelFailed(String),
    /// Assertion failed with a descriptive message
    AssertionFailed(String),
    /// Generic I/O error
    Io(std::io::Error),
    /// JSON serialization/deserialization error
    Json(serde_json::Error),
}

impl fmt::Display for AnvilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnvilError::SnapshotLoad(msg)    => write!(f, "Failed to load snapshot: {}", msg),
            AnvilError::SnapshotWrite(msg)   => write!(f, "Failed to write snapshot: {}", msg),
            AnvilError::AccountNotFound(id)  => write!(f, "Account not found: {}", id),
            AnvilError::ContractNotFound(alias) => write!(f, "Contract not found: {}", alias),
            AnvilError::TimeTravelFailed(msg) => write!(f, "Time travel failed: {}", msg),
            AnvilError::AssertionFailed(msg) => write!(f, "Assertion failed: {}", msg),
            AnvilError::Io(e)                => write!(f, "I/O error: {}", e),
            AnvilError::Json(e)              => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for AnvilError {}

impl From<std::io::Error> for AnvilError {
    fn from(e: std::io::Error) -> Self {
        AnvilError::Io(e)
    }
}

impl From<serde_json::Error> for AnvilError {
    fn from(e: serde_json::Error) -> Self {
        AnvilError::Json(e)
    }
}
