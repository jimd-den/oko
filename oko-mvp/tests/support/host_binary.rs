//! Drive the real `oko-host` binary over its stdio IPC.
//!
//! This is the outer-boundary smoke test for the composition root: argument
//! parsing, engine selection and the stdio transport are glue, and glue is only
//! proven by starting the real thing (AGENTS.md §2.1, "declarative glue").
//!
//! Deterministic: no network, no clock, and the host exits at `host.shutdown`.

use oko_core::ipc::message::Message;
use std::io::Write;
use std::process::{Command, Stdio};

/// Run `oko-host` with `args`, feed it `script`, and return every message it
/// answered. The host binary path comes from cargo, so this only works from an
/// integration test.
pub fn run_host_binary(args: &[&str], script: &[Message]) -> Vec<Message> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_oko-host"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("oko-host must start");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for msg in script {
            writeln!(stdin, "{}", msg.to_line()).expect("write request");
        }
    }

    let output = child.wait_with_output().expect("oko-host must exit");
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter(|line| line.starts_with('{'))
        .map(|line| Message::from_line(line).expect("host must emit valid IPC lines"))
        .collect()
}

/// The payload of the host's successful reply to `route`.
pub fn reply_for<'a>(messages: &'a [Message], route: &str) -> Option<&'a Message> {
    let reply_route = format!("{}.reply", route);
    messages
        .iter()
        .find(|m| m.route == reply_route && m.error.is_none())
}
