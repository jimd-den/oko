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
    UiModeDesktop,
    RenderDemo,
    RenderStats,
    RenderCapture { path: String },
}

/// A host API response.
#[derive(Debug, Clone)]
pub enum HostApiResponse {
    TerminalCreated {
        session_id: String,
    },
    TerminalOutput {
        output: String,
    },
    TerminalExitStatus {
        code: Option<i32>,
    },
    AppList {
        sessions: Vec<String>,
    },
    AppLaunched {
        id: String,
    },
    AppStopped {
        id: String,
    },
    SystemStatus {
        core: String,
        terminal: String,
    },
    BrowserOpened {
        url: String,
    },
    ConfigRead {
        engine: String,
        terminal_visible: bool,
    },
    UiMode {
        mode: String,
    },
    RenderDemo {
        output: String,
    },
    RenderStats {
        stats: String,
    },
    RenderCapture {
        path: String,
    },
    Error {
        code: String,
        message: String,
    },
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
        // Sensitive system operations: change UI mode and render.
        HostApiRequest::UiModeDesktop => vec![Capability::SystemConfigure],
        HostApiRequest::RenderDemo => vec![Capability::GraphicsCreate],
        HostApiRequest::RenderStats => vec![Capability::GraphicsCreate],
        HostApiRequest::RenderCapture { .. } => vec![Capability::GraphicsCreate],
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
mod tests;
