//! Handler: host.terminal.create / host.terminal.read.
//! Small composable functions, single responsibility each.

use crate::domain::session::SessionId;
use crate::ipc::json::object;
use crate::ipc::server::Core;

pub fn handle_create(server: &mut Core) -> Result<crate::ipc::json::JsonValue, String> {
    let id = SessionId(format!("pty-{}", next_id()));
    server
        .sessions
        .create_terminal(server.terminal.as_ref(), id.clone())
        .map_err(|e| format!("{:?}", e))?;
    Ok(object(vec![("session_id".into(), str(&id.0))]))
}

/// Send user input to a terminal session (SRD TERM-002, UI-002).
///
/// The desktop shell types through this route; the session lives in the host,
/// so a UI reload does not end the shell (TERM-003).
pub fn handle_write(
    server: &mut Core,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let session_id = payload_string(payload, "session_id")?;
    let text = payload_string(payload, "text")?;
    let pty_session = server
        .sessions
        .pty(&SessionId(session_id))
        .ok_or("session not found")?
        .clone();
    server
        .terminal
        .write_input(&pty_session, text.as_bytes())
        .map_err(|e| format!("{:?}", e))?;
    Ok(object(vec![(
        "written".into(),
        crate::ipc::json::num(text.len() as f64),
    )]))
}

pub fn handle_read(
    server: &mut Core,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let session_id = payload_string(payload, "session_id")?;
    let pty_session = server
        .sessions
        .pty(&SessionId(session_id))
        .ok_or("session not found")?
        .clone();
    let output = server
        .terminal
        .read_output(&pty_session)
        .map_err(|e| format!("{:?}", e))?;
    Ok(object(vec![("output".into(), str(&output))]))
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

fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(100);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_create_returns_session_id() {
        let mut server = test_server();
        let result = handle_create(&mut server).unwrap();
        let session_id = crate::ipc::json::get(&result, "session_id")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert!(session_id.starts_with("pty-"));
    }

    #[test]
    fn handle_read_missing_session_errors() {
        let mut server = test_server();
        let payload = crate::ipc::json::object(vec![("session_id".into(), str("nonexistent"))]);
        let result = handle_read(&mut server, &payload);
        assert!(result.is_err());
    }

    #[test]
    fn handle_write_sends_input_to_the_session() {
        let mut server = test_server();
        let created = handle_create(&mut server).unwrap();
        let session_id = crate::ipc::json::get(&created, "session_id")
            .and_then(crate::ipc::json::as_str)
            .unwrap()
            .to_string();

        let payload = crate::ipc::json::object(vec![
            ("session_id".into(), str(&session_id)),
            ("text".into(), str("echo written\n")),
        ]);
        handle_write(&mut server, &payload).unwrap();

        let read = handle_read(
            &mut server,
            &crate::ipc::json::object(vec![("session_id".into(), str(&session_id))]),
        )
        .unwrap();
        let output = crate::ipc::json::get(&read, "output")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        // A real PTY echoes the input back, so the output is what a terminal
        // shows rather than the command's stdout alone.
        assert!(output.contains("written"), "output was {:?}", output);
    }

    #[test]
    fn handle_write_to_an_unknown_session_errors() {
        let mut server = test_server();
        let payload = crate::ipc::json::object(vec![
            ("session_id".into(), str("nonexistent")),
            ("text".into(), str("echo hi\n")),
        ]);
        assert!(handle_write(&mut server, &payload).is_err());
    }

    #[test]
    fn next_id_increments() {
        let _ = next_id();
        let first = next_id();
        let second = next_id();
        assert!(second > first);
    }

    fn test_server() -> Core {
        use crate::application::session_service::SessionService;
        use crate::browser::adapters::mock::MockBrowserEngineAdapter;
        use crate::browser::ports::engine::BrowserEnginePort;
        use crate::domain::capability::{Capability, CapabilityPolicy};
        use crate::ports::process::ProcessPort;
        use crate::ports::terminal::TerminalPort;

        let process: Box<dyn ProcessPort> =
            Box::new(crate::adapters::linux_process::LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> =
            Box::new(crate::adapters::portable_pty::PortablePtyAdapter::new());
        let sessions = SessionService::new();
        let policy = CapabilityPolicy::new().with(Capability::TerminalCreate);
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());

        Core::new(
            process,
            terminal,
            sessions,
            policy,
            browser,
        )
    }
}
