//! Handler: host.browser.open / host.browser.engine / host.browser.capabilities.
//! Small composable functions, single responsibility each.

use crate::browser::ports::engine::BrowserConfig;
use crate::ipc::json::object;
use crate::ipc::server::IpcServer;
use crate::ipc::transport::IpcTransport;

pub fn handle_open<T: IpcTransport>(
    server: &mut IpcServer<T>,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let url = payload_string(payload, "url")?;
    if let Some(reason) = &server.engine_unavailable {
        return Err(format!("browser engine unavailable: {}", reason));
    }
    let config = BrowserConfig {
        profile: "default".into(),
        url: Some(url.clone()),
        off_the_record: false,
    };
    let _session = server.browser.create_session(config).map_err(|e| format!("{:?}", e))?;
    Ok(object(vec![("url".into(), str(&url))]))
}

/// Report engine identity, version and state (SRD OBS-003, ENG-008).
///
/// `state` is `ready` or `unavailable`; an unavailable engine also carries the
/// `reason`, so a caller can tell a missing engine from a working one instead
/// of receiving a single ambiguous OK.
pub fn handle_engine<T: IpcTransport>(server: &IpcServer<T>) -> Result<crate::ipc::json::JsonValue, String> {
    let mut fields = vec![
        ("engine".into(), str(server.browser.engine_name())),
        ("version".into(), str(server.browser.engine_version())),
        ("experimental".into(), bool(server.browser.is_experimental())),
    ];
    match &server.engine_unavailable {
        Some(reason) => {
            fields.push(("state".into(), str("unavailable")));
            fields.push(("reason".into(), str(reason)));
        }
        None => fields.push(("state".into(), str("ready"))),
    }
    // A crash after a good start leaves the engine "ready" but not trustworthy;
    // reporting the reason is what makes it visible (SRD OBS-003, ENG-004).
    if let Some(crash) = server.browser.last_crash_reason() {
        fields.push(("last_crash".into(), str(&crash)));
    }
    Ok(object(fields))
}

pub fn handle_capabilities<T: IpcTransport>(server: &IpcServer<T>) -> Result<crate::ipc::json::JsonValue, String> {
    let caps = server.browser.capabilities();
    Ok(object(vec![
        ("supports_js".into(), bool(caps.supports_js)),
        ("supports_downloads".into(), bool(caps.supports_downloads)),
        ("supports_permissions".into(), bool(caps.supports_permissions)),
        ("is_experimental".into(), bool(caps.is_experimental)),
    ]))
}

fn str(v: &str) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::String(v.into())
}

fn bool(b: bool) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::Bool(b)
}

fn payload_string(payload: &crate::ipc::json::JsonValue, key: &str) -> Result<String, String> {
    match crate::ipc::json::get(payload, key).and_then(crate::ipc::json::as_str) {
        Some(s) => Ok(s.into()),
        None => Err(format!("missing payload field: {}", key)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_open_returns_url() {
        let mut server = test_server();
        let payload = crate::ipc::json::object(vec![("url".into(), str("https://example.com"))]);
        let result = handle_open(&mut server, &payload).unwrap();
        let url = crate::ipc::json::get(&result, "url").and_then(crate::ipc::json::as_str).unwrap();
        assert_eq!(url, "https://example.com");
    }

    #[test]
    fn handle_engine_returns_mock() {
        let server = test_server();
        let result = handle_engine(&server).unwrap();
        let engine = crate::ipc::json::get(&result, "engine").and_then(crate::ipc::json::as_str).unwrap();
        assert_eq!(engine, "mock");
    }

    #[test]
    fn handle_engine_reports_ready_state_before_any_failure() {
        let server = test_server();
        let result = handle_engine(&server).unwrap();
        let state = crate::ipc::json::get(&result, "state").and_then(crate::ipc::json::as_str).unwrap();
        assert_eq!(state, "ready");
    }

    #[test]
    fn handle_capabilities_returns_flags() {
        let server = test_server();
        let result = handle_capabilities(&server).unwrap();
        let js = crate::ipc::json::get(&result, "supports_js").and_then(crate::ipc::json::as_bool).unwrap();
        assert!(js);
    }

    fn str(v: &str) -> crate::ipc::json::JsonValue {
        crate::ipc::json::JsonValue::String(v.into())
    }

    fn test_server() -> IpcServer<crate::ipc::transport::StdioTransport> {
        use crate::application::session_service::SessionService;
        use crate::domain::capability::{Capability, CapabilityPolicy};
        use crate::browser::ports::engine::BrowserEnginePort;
        use crate::browser::adapters::mock::MockBrowserEngineAdapter;
        use crate::ports::process::ProcessPort;
        use crate::ports::terminal::TerminalPort;
        use std::env::temp_dir;

        let process: Box<dyn ProcessPort> = Box::new(crate::adapters::linux_process::LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> = Box::new(crate::adapters::linux_pty::LinuxPtyAdapter::new());
        let sessions = SessionService::new();
        let policy = CapabilityPolicy::new().with(Capability::BrowserNavigate);
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let root = temp_dir().join("oko-browser-test");
        let _ = std::fs::create_dir_all(root.join("toolchain"));

        IpcServer::new_stdio(
            process, terminal, sessions, policy, browser, &root,
        )
    }
}
