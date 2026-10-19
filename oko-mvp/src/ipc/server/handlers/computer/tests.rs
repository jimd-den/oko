//! What the computer does with each kind of request, driven through the
//! handler exactly as the shell drives it.

use super::*;
use crate::application::session_service::SessionService;
use crate::browser::adapters::{blink_install::BlinkInstaller, mock::MockBrowserEngineAdapter};
use crate::domain::capability::{Capability, CapabilityPolicy};
use crate::ipc::json;
use crate::ports::process::ProcessPort;
use crate::ports::terminal::TerminalPort;

fn core() -> Core {
    let process: Box<dyn ProcessPort> =
        Box::new(crate::adapters::linux_process::LinuxProcessAdapter::new());
    let terminal: Box<dyn TerminalPort> =
        Box::new(crate::adapters::portable_pty::PortablePtyAdapter::new());
    let root = std::env::temp_dir().join("oko-computer-test");
    let _ = std::fs::create_dir_all(root.join("toolchain"));
    Core::new(
        process,
        terminal,
        SessionService::new(),
        CapabilityPolicy::new().with(Capability::TerminalCreate),
        Box::new(MockBrowserEngineAdapter::new()),
        BlinkInstaller::new(&root),
    )
}

fn ask(core: &mut Core, text: &str) -> String {
    let payload = json::object(vec![("text".into(), str(text))]);
    let reply = handle_ask(core, &payload).unwrap();
    json::get(&reply, "reply")
        .and_then(json::as_str)
        .unwrap()
        .to_string()
}

#[test]
fn a_status_request_is_answered_with_the_state() {
    let mut core = core();
    let reply = ask(&mut core, "computer, status");
    assert!(reply.contains("Core is running"), "reply was {:?}", reply);
}

#[test]
fn a_command_is_answered_with_its_output_and_not_with_the_request() {
    let mut core = core();
    // The shell echoes what it is given and prints a prompt when it is ready
    // again. Neither is an answer, so neither is in the reply.
    assert_eq!(ask(&mut core, "echo hello"), "hello");
}

#[test]
fn a_command_with_no_output_says_so() {
    let mut core = core();
    assert_eq!(ask(&mut core, "true"), "Command completed with no output.");
}

#[test]
fn a_command_still_returns_the_session_id() {
    let mut core = core();
    let payload = json::object(vec![("text".into(), str("echo one"))]);
    let reply = handle_ask(&mut core, &payload).unwrap();
    assert_eq!(
        json::get(&reply, "session_id").and_then(json::as_str),
        Some(DEFAULT_SESSION)
    );
}

#[test]
fn consecutive_requests_share_one_session() {
    let mut core = core();
    let payload = json::object(vec![("text".into(), str("echo one"))]);
    let first = handle_ask(&mut core, &payload).unwrap();
    let id = json::get(&first, "session_id")
        .and_then(json::as_str)
        .unwrap()
        .to_string();
    assert_eq!(id, DEFAULT_SESSION);
    assert_eq!(core.sessions.list().len(), 1);
}

#[test]
fn silence_gets_a_ready_not_an_error() {
    let mut core = core();
    assert_eq!(ask(&mut core, "   "), "Ready.");
}

#[test]
fn a_missing_text_field_is_refused() {
    let mut core = core();
    let payload = json::object(vec![]);
    assert!(handle_ask(&mut core, &payload).is_err());
}
