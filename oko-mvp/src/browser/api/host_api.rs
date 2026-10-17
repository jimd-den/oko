//! HostApiPort — typed, permission-checked host API for the desktop UI (SRD UI-008).
//!
//! Capabilities are checked by the native service on every sensitive request.
//! A declaration in a UI document is not sufficient to grant authority (SRD §8.3).

use crate::domain::capability::{Capability, CapabilityPolicy};
use crate::domain::session::SessionId;

/// A host API request from the desktop UI.
#[derive(Debug, Clone)]
pub enum HostApiRequest {
    TerminalCreate,
    TerminalWrite { session_id: String, text: String },
    TerminalRead { session_id: String },
    TerminalClose { session_id: String },
    AppList,
    AppLaunch { id: String },
    AppStop { id: String },
    SystemStatus,
    BrowserOpen { url: String },
    ConfigRead,
}

/// A host API response.
#[derive(Debug, Clone)]
pub enum HostApiResponse {
    TerminalCreated { session_id: String },
    TerminalOutput { output: String },
    TerminalExitStatus { code: Option<i32> },
    AppList { sessions: Vec<String> },
    AppLaunched { id: String },
    AppStopped { id: String },
    SystemStatus { core: String, terminal: String },
    BrowserOpened { url: String },
    ConfigRead { engine: String, terminal_visible: bool },
    Error { code: String, message: String },
}

/// Capabilities required by each host API request.
pub fn capabilities_required(req: &HostApiRequest) -> Vec<Capability> {
    match req {
        HostApiRequest::TerminalCreate => vec![Capability::TerminalCreate],
        HostApiRequest::TerminalWrite { .. } | HostApiRequest::TerminalRead { .. } => {
            vec![Capability::TerminalCreate]
        }
        HostApiRequest::TerminalClose { .. } => vec![Capability::TerminalCreate],
        HostApiRequest::AppList => vec![],
        HostApiRequest::AppLaunch { .. } | HostApiRequest::AppStop { .. } => {
            vec![Capability::ProcessSpawn]
        }
        HostApiRequest::SystemStatus => vec![],
        HostApiRequest::BrowserOpen { .. } => vec![Capability::BrowserNavigate],
        HostApiRequest::ConfigRead => vec![],
    }
}

/// Validate a host API request against the capability policy. Pure, composable.
pub fn validate_request(req: &HostApiRequest, policy: &CapabilityPolicy) -> Result<(), String> {
    let required = capabilities_required(req);
    if policy.allows(&required) {
        Ok(())
    } else {
        let names: Vec<String> = required.iter().map(|c| format!("{:?}", c)).collect();
        Err(format!("permission denied: requires {}", names.join(", ")))
    }
}

/// The Host API port: handle a request against the core services.
pub trait HostApiPort: Send + Sync {
    fn handle(
        &self,
        request: HostApiRequest,
        ctx: &HostApiContext,
    ) -> Result<HostApiResponse, String>;
}

/// Context passed to host API handlers.
pub struct HostApiContext<'a> {
    pub policy: &'a CapabilityPolicy,
    pub sessions: &'a mut dyn TerminalSessionService,
}

/// Abstraction over terminal session management for the host API.
pub trait TerminalSessionService: Send + Sync {
    fn create_terminal(&mut self) -> Result<SessionId, String>;
    fn write_input(&mut self, session_id: &SessionId, text: &str) -> Result<(), String>;
    fn read_output(&mut self, session_id: &SessionId) -> Result<String, String>;
    fn exit_status(&self, session_id: &SessionId) -> Result<Option<i32>, String>;
    fn list_sessions(&self) -> Vec<SessionId>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_terminal_create_requires_terminal_create_cap() {
        let policy = CapabilityPolicy::new().with(Capability::TerminalCreate);
        assert!(validate_request(&HostApiRequest::TerminalCreate, &policy).is_ok());

        let empty = CapabilityPolicy::new();
        assert!(validate_request(&HostApiRequest::TerminalCreate, &empty).is_err());
    }

    #[test]
    fn validate_app_launch_requires_process_spawn_cap() {
        let policy = CapabilityPolicy::new().with(Capability::ProcessSpawn);
        assert!(validate_request(
            &HostApiRequest::AppLaunch { id: "calc".into() },
            &policy,
        )
        .is_ok());

        let empty = CapabilityPolicy::new();
        assert!(validate_request(
            &HostApiRequest::AppLaunch { id: "calc".into() },
            &empty,
        )
        .is_err());
    }

    #[test]
    fn validate_browser_open_requires_browser_navigate_cap() {
        let policy = CapabilityPolicy::new().with(Capability::BrowserNavigate);
        assert!(validate_request(
            &HostApiRequest::BrowserOpen { url: "https://example.com".into() },
            &policy,
        )
        .is_ok());

        let empty = CapabilityPolicy::new();
        assert!(validate_request(
            &HostApiRequest::BrowserOpen { url: "https://example.com".into() },
            &empty,
        )
        .is_err());
    }

    #[test]
    fn validate_status_needs_no_capability() {
        let empty = CapabilityPolicy::new();
        assert!(validate_request(
            &HostApiRequest::SystemStatus,
            &empty,
        )
        .is_ok());
    }
}
