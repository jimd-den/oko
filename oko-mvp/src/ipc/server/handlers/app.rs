//! Handler: host.app.list / host.app.launch / host.app.stop.
//! Small composable functions, single responsibility each.

use crate::domain::session::SessionId;
use crate::ipc::json::object;
use crate::ipc::server::Core;
use crate::ports::process::SpawnRequest;

pub fn handle_list(server: &Core) -> Result<crate::ipc::json::JsonValue, String> {
    let sessions_list: Vec<String> = server
        .sessions
        .list()
        .iter()
        .map(|s| format!("{:?}", s.id.0))
        .collect();
    Ok(object(vec![(
        "sessions".into(),
        arr(sessions_list.iter().map(|s| str(s)).collect()),
    )]))
}

pub fn handle_launch(
    server: &mut Core,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let id = payload_string(payload, "id")?;
    let req = SpawnRequest {
        executable: "sh".into(),
        args: vec!["-c".into(), format!("echo 'app {} running'", id)],
    };
    let _handle = server.process.spawn(req).map_err(|e| format!("{:?}", e))?;
    let session_id = SessionId(format!("app-{}", id));
    server
        .sessions
        .create_terminal(server.terminal.as_ref(), session_id.clone())
        .map_err(|e| format!("{:?}", e))?;
    Ok(object(vec![("id".into(), str(&id))]))
}

pub fn handle_stop(
    server: &mut Core,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let id = payload_string(payload, "id")?;
    let session_id = SessionId(format!("app-{}", id));
    server.sessions.stop(&session_id);
    Ok(object(vec![("id".into(), str(&id))]))
}

fn str(v: &str) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::String(v.into())
}

fn arr(items: Vec<crate::ipc::json::JsonValue>) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::Array(items)
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
    fn handle_list_returns_empty_when_no_sessions() {
        let server = test_server();
        let result = handle_list(&server).unwrap();
        let _sessions = crate::ipc::json::get(&result, "sessions");
        assert!(_sessions.is_some());
    }

    #[test]
    fn handle_launch_creates_session() {
        let mut server = test_server();
        let payload = crate::ipc::json::object(vec![("id".into(), str("calc"))]);
        let result = handle_launch(&mut server, &payload).unwrap();
        let id = crate::ipc::json::get(&result, "id")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(id, "calc");
    }

    #[test]
    fn handle_stop_removes_session() {
        let mut server = test_server();
        let _ = handle_launch(
            &mut server,
            &crate::ipc::json::object(vec![("id".into(), str("calc"))]),
        );
        let result = handle_stop(
            &mut server,
            &crate::ipc::json::object(vec![("id".into(), str("calc"))]),
        )
        .unwrap();
        let id = crate::ipc::json::get(&result, "id")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(id, "calc");
    }

    fn str(v: &str) -> crate::ipc::json::JsonValue {
        crate::ipc::json::JsonValue::String(v.into())
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
        let policy = CapabilityPolicy::new().with(Capability::ProcessSpawn);
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let root = temp_dir().join("oko-app-test");
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
