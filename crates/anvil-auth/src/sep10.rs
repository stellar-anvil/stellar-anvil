use soroban_sdk::{Address, Env};
use colored::Colorize;
use std::time::{SystemTime, UNIX_EPOCH};

/// Errors that can occur during SEP-10 challenge/response simulation.
#[derive(Debug, Clone, PartialEq)]
pub enum Sep10Error {
    /// Challenge has expired (current time > issued_at + timeout_secs)
    ChallengeExpired,
    /// The signing address does not match the challenge address
    AddressMismatch,
    /// The session token has already been used (replay protection)
    TokenReplayed,
    /// The challenge was never issued for this address
    NoChallengeIssued,
}

impl std::fmt::Display for Sep10Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sep10Error::ChallengeExpired    => write!(f, "SEP-10 challenge has expired"),
            Sep10Error::AddressMismatch     => write!(f, "signing address does not match challenge"),
            Sep10Error::TokenReplayed       => write!(f, "session token has already been used"),
            Sep10Error::NoChallengeIssued   => write!(f, "no challenge was issued for this address"),
        }
    }
}

/// A simulated SEP-10 challenge transaction.
#[derive(Debug, Clone)]
pub struct Sep10Challenge {
    pub address: String,
    pub nonce: [u8; 32],
    pub issued_at: u64,
    pub timeout_secs: u64,
    pub consumed: bool,
}

impl Sep10Challenge {
    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn new(address: impl Into<String>, timeout_secs: u64) -> Self {
        let now = Self::now_secs();
        let mut nonce = [0u8; 32];
        let ts_bytes = now.to_le_bytes();
        nonce[..8].copy_from_slice(&ts_bytes);
        Self {
            address: address.into(),
            nonce,
            issued_at: now,
            timeout_secs,
            consumed: false,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = Self::now_secs();
        now > self.issued_at + self.timeout_secs
    }
}

/// A simulated SEP-10 session token — issued after a successful challenge/response.
#[derive(Debug, Clone)]
pub struct Sep10Session {
    pub address: String,
    pub token: String,
    pub expires_at: u64,
    pub used: bool,
}

impl Sep10Session {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now > self.expires_at
    }
}

/// A mock SEP-10 server for testing authentication flows.
///
/// # Example
/// ```ignore
/// let mut server = MockSep10Server::new();
/// let challenge = server.issue_challenge("GADDRESS...", 300)?;
/// let session   = server.verify_challenge(&challenge, "GADDRESS...")?;
/// assert!(server.validate_session(&session.token).is_ok());
/// assert!(server.validate_session(&session.token).is_err()); // replay
/// ```
pub struct MockSep10Server {
    pending_challenges: std::collections::HashMap<String, Sep10Challenge>,
    issued_sessions: std::collections::HashMap<String, Sep10Session>,
    session_duration_secs: u64,
    verbose: bool,
}

impl MockSep10Server {
    pub fn new() -> Self {
        Self {
            pending_challenges: std::collections::HashMap::new(),
            issued_sessions: std::collections::HashMap::new(),
            session_duration_secs: 86_400,
            verbose: false,
        }
    }

    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    pub fn with_session_duration(mut self, secs: u64) -> Self {
        self.session_duration_secs = secs;
        self
    }

    pub fn issue_challenge(
        &mut self,
        address: impl Into<String>,
        timeout_secs: u64,
    ) -> Result<Sep10Challenge, Sep10Error> {
        let address = address.into();
        let challenge = Sep10Challenge::new(&address, timeout_secs);
        if self.verbose {
            println!(
                "{} SEP-10 challenge issued for {} (expires in {}s)",
                "🔐".cyan(),
                address,
                timeout_secs,
            );
        }
        self.pending_challenges.insert(address, challenge.clone());
        Ok(challenge)
    }

    pub fn verify_challenge(
        &mut self,
        challenge: &Sep10Challenge,
        signing_address: &str,
    ) -> Result<Sep10Session, Sep10Error> {
        let stored = self
            .pending_challenges
            .get(&challenge.address)
            .ok_or(Sep10Error::NoChallengeIssued)?;

        if stored.consumed {
            return Err(Sep10Error::TokenReplayed);
        }
        if stored.is_expired() {
            return Err(Sep10Error::ChallengeExpired);
        }
        if signing_address != challenge.address {
            return Err(Sep10Error::AddressMismatch);
        }

        self.pending_challenges
            .get_mut(&challenge.address)
            .unwrap()
            .consumed = true;

        let token = format!("sep10.{}.{}", signing_address, challenge.issued_at);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let session = Sep10Session {
            address: signing_address.to_string(),
            token: token.clone(),
            expires_at: now + self.session_duration_secs,
            used: false,
        };

        if self.verbose {
            println!(
                "{} SEP-10 session issued for {} (expires in {}s)",
                "✓".green(),
                signing_address,
                self.session_duration_secs,
            );
        }

        self.issued_sessions.insert(token, session.clone());
        Ok(session)
    }

    pub fn validate_session(&mut self, token: &str) -> Result<&Sep10Session, Sep10Error> {
        let session = self
            .issued_sessions
            .get_mut(token)
            .ok_or(Sep10Error::NoChallengeIssued)?;

        if session.used {
            return Err(Sep10Error::TokenReplayed);
        }
        if session.is_expired() {
            return Err(Sep10Error::ChallengeExpired);
        }

        session.used = true;

        if self.verbose {
            println!(
                "{} SEP-10 token validated for {}",
                "✓".green(),
                session.address,
            );
        }

        Ok(self.issued_sessions.get(token).unwrap())
    }

    /// Assert a full SEP-10 flow succeeds for a given address.
    pub fn assert_auth_flow(&mut self, address: &str) {
        let challenge = self
            .issue_challenge(address, 300)
            .expect("anvil-auth: failed to issue SEP-10 challenge");
        let session = self
            .verify_challenge(&challenge, address)
            .expect("anvil-auth: SEP-10 challenge verification failed");
        self.validate_session(&session.token)
            .expect("anvil-auth: SEP-10 session validation failed");
        println!(
            "{} SEP-10 full auth flow passed for {}",
            "✓".green().bold(),
            address
        );
    }
}

impl Default for MockSep10Server {
    fn default() -> Self {
        Self::new()
    }
}
