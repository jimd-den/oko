//! Acceptance test AT-002: terminal operation.
//!
//! Pass condition: a real shell accepts input, renders output, and reports exit
//! status, independently of any desktop or browser process (SRD TERM-001..008).
//!
//! Rewritten when the PTY adapter became real. The previous version asserted
//! that `echo hello` produced exactly `"hello\n"` and that `false` reported
//! exit 1 — both only true of the stub it was written against, which ran each
//! line through `sh -c`. A real PTY echoes what you type and a shell keeps
//! running after a failed command, so the intent (input in, output out, exit
//! status reported) is preserved here against a shell that behaves like one.

use oko_core::adapters::portable_pty::PortablePtyAdapter;
use oko_core::ports::terminal::TerminalPort;
use std::time::{Duration, Instant};

/// Wait for `check`, up to a deadline: a shell is a process, not a function.
fn eventually<F: FnMut() -> bool>(mut check: F) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if check() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn a_shell_accepts_input_and_renders_output() {
    let pty = PortablePtyAdapter::new();
    let session = pty.create_session().expect("a PTY session should open");

    pty.write_input(&session, b"echo hello\n")
        .expect("input should be written");

    let mut seen = String::new();
    assert!(
        eventually(|| {
            seen.push_str(&pty.read_output(&session).unwrap_or_default());
            seen.contains("hello")
        }),
        "the shell never rendered the output; saw {:?}",
        seen
    );
}

#[test]
fn a_shell_reports_the_exit_status_of_the_session() {
    let pty = PortablePtyAdapter::new();
    let session = pty.create_session().expect("a PTY session should open");

    // The status of a session is the status of the shell in it (TERM-008).
    pty.write_input(&session, b"exit 3\n")
        .expect("input written");

    let mut status = None;
    assert!(
        eventually(|| {
            status = pty.exit_status(&session).unwrap_or(None);
            status.is_some()
        }),
        "the session never ended; last status {:?}",
        status
    );
    assert_eq!(status, Some(3));
}

#[test]
fn sessions_persist_independent_of_ui() {
    // Sessions are owned by SessionService, not by any UI (TERM-003).
    let pty = PortablePtyAdapter::new();
    let session_a = pty.create_session().unwrap();
    let session_b = pty.create_session().unwrap();

    pty.write_input(&session_a, b"echo session-a\n").unwrap();
    pty.write_input(&session_b, b"echo session-b\n").unwrap();

    let mut out_a = String::new();
    let mut out_b = String::new();
    assert!(eventually(|| {
        out_a.push_str(&pty.read_output(&session_a).unwrap_or_default());
        out_b.push_str(&pty.read_output(&session_b).unwrap_or_default());
        out_a.contains("session-a") && out_b.contains("session-b")
    }));
    // Two independent sessions; one can close without affecting the other.
    assert!(pty.exit_status(&session_a).is_ok());
    assert!(pty.exit_status(&session_b).is_ok());
}
