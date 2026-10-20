//! BrowserEnginePort — the ỌKỌ browser interface (SRD §5.2).
//!
//! Hides engine-specific objects (DOM types, script-engine handles, rendering
//! state). Each backend is a separate adapter. Exactly one engine backend
//! serves one browser instance (SRD ENG-001).

use crate::domain::session::SessionId;

/// Opaque, engine-neutral browser session handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BrowserSession(pub SessionId);

/// Browser configuration handed to the engine adapter.
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub profile: String,
    pub url: Option<String>,
    pub off_the_record: bool,
}

/// Errors from the browser engine abstraction (engine-neutral).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserError {
    EngineUnavailable(String),
    SessionCreateFailed(String),
    SessionNotFound,
    NavigationFailed(String),
    ScriptFailed(String),
    Crash(String),
}

/// Capabilities a backend declares (SRD ENG-003).
#[derive(Debug, Clone, Default)]
pub struct BackendCapabilities {
    pub supports_js: bool,
    pub supports_downloads: bool,
    pub supports_permissions: bool,
    pub is_experimental: bool,
}

/// The stable ỌKỌ browser interface. Engine backends implement this.
pub trait BrowserEnginePort: Send + Sync {
    /// Engine initialization and shutdown.
    fn initialize(&self) -> Result<(), BrowserError>;
    fn shutdown(&self) -> Result<(), BrowserError>;

    /// Engine identity (SRD OBS-003).
    fn engine_name(&self) -> &str;
    fn engine_version(&self) -> &str;

    /// Whether this backend is visibly experimental (SRD ENG-009).
    fn is_experimental(&self) -> bool;

    /// Declared features and limitations (SRD ENG-003).
    fn capabilities(&self) -> BackendCapabilities;

    /// Browser context / profile creation.
    fn create_session(&self, config: BrowserConfig) -> Result<BrowserSession, BrowserError>;

    /// Navigation and redirects.
    fn navigate(&self, session: &BrowserSession, url: &str) -> Result<(), BrowserError>;

    /// Close a session.
    fn close_session(&self, session: &BrowserSession) -> Result<(), BrowserError>;

    /// Execute engine-neutral script in a session (for host API bridge).
    fn execute_script(
        &self,
        session: &BrowserSession,
        script: &str,
    ) -> Result<String, BrowserError>;

    /// Crash notification and recovery (SRD ENG-004).
    fn last_crash_reason(&self) -> Option<String>;
}
