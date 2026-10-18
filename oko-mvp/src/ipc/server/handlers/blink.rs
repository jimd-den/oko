//! Handler: blink.install / blink.status / blink.fetch-checksum.
//! Small composable functions, single responsibility each.

use crate::browser::adapters::blink_install::{InstallResult, InstallState};
use crate::ipc::json::object;
use crate::ipc::server::Core;

pub fn handle_install(server: &mut Core) -> Result<crate::ipc::json::JsonValue, String> {
    match server.blink_installer.install() {
        InstallResult::Ok => Ok(object(vec![
            ("state".into(), str("installed")),
            ("engine".into(), str("blink")),
        ])),
        InstallResult::AlreadyInstalled => Ok(object(vec![
            ("state".into(), str("already-installed")),
            ("engine".into(), str("blink")),
        ])),
        InstallResult::Failed(e) => Err(e),
        InstallResult::NotInstalled => Err("blink not installed".into()),
    }
}

pub fn handle_status(server: &Core) -> Result<crate::ipc::json::JsonValue, String> {
    let state: String = match server.blink_installer.status() {
        InstallState::NotInstalled => "not-installed".to_string(),
        InstallState::Downloaded => "downloaded".to_string(),
        InstallState::Built => "built".to_string(),
    };
    Ok(object(vec![("state".into(), str(&state))]))
}

pub fn handle_fetch_checksum(
    server: &Core,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let path = payload_string(payload, "path")?;
    match server
        .blink_installer
        .fetch_checksum(std::path::Path::new(&path))
    {
        InstallResult::Failed(msg) => {
            if let Some(digest) = msg.split("is ").nth(1).and_then(|s| s.split(' ').next()) {
                Ok(object(vec![("digest".into(), str(digest))]))
            } else {
                Err(msg)
            }
        }
        _ => Err("unexpected".into()),
    }
}

fn str(v: &str) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::String(v.into())
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
    fn handle_status_not_installed() {
        let server = test_server();
        let result = handle_status(&server).unwrap();
        let state = crate::ipc::json::get(&result, "state")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(state, "not-installed");
    }

    #[test]
    fn handle_install_fails_without_checksum() {
        let mut server = test_server();
        let result = handle_install(&mut server);
        assert!(result.is_err());
    }

    fn test_server() -> Core {
        use crate::application::session_service::SessionService;
        use crate::browser::adapters::blink_install::BlinkInstaller;
        use crate::browser::adapters::mock::MockBrowserEngineAdapter;
        use crate::browser::ports::engine::BrowserEnginePort;
        use crate::domain::capability::{Capability, CapabilityPolicy};
        use crate::ports::process::ProcessPort;
        use crate::ports::terminal::TerminalPort;
        use std::env::temp_dir;

        let process: Box<dyn ProcessPort> =
            Box::new(crate::adapters::linux_process::LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> =
            Box::new(crate::adapters::linux_pty::LinuxPtyAdapter::new());
        let sessions = SessionService::new();
        let policy = CapabilityPolicy::new().with(Capability::SystemConfigure);
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let root = temp_dir().join("oko-blink-handler-test");
        let _ = std::fs::create_dir_all(root.join("toolchain"));

        Core::new(
            process,
            terminal,
            sessions,
            policy,
            browser,
            BlinkInstaller::new(&root),
        )
    }
}
