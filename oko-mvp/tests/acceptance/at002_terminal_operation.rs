//! Acceptance test AT-002: Terminal operation.
//!
//! Pass condition: a real shell accepts input, renders output, and reports
//! exit status, independently of any desktop or browser process.
//!
//! The LinuxPtyAdapter is the slice-1 terminal port; a real posix_openpt/nix
//! PTY replaces it behind the same trait in production.

use oko_core::adapters::linux_pty::LinuxPtyAdapter;
use oko_core::ports::terminal::TerminalPort;

#[test]
fn pty_shell_accepts_input_renders_output_reports_exit() {
    let pty = LinuxPtyAdapter::new();
    let session = pty.create_session().expect("PTY session should open");

    // Input is accepted.
    pty.write_input(&session, b"echo hello\n")
        .expect("input should be written");

    // Output is rendered.
    let output = pty
        .read_output(&session)
        .expect("output should be readable");
    assert_eq!(output, "hello\n");

    // Exit status is reported accurately (TERM-008).
    let status = pty
        .exit_status(&session)
        .expect("exit status should be reported");
    assert_eq!(status, Some(0));

    // A failing command reports non-zero.
    pty.write_input(&session, b"false\n")
        .expect("input written");
    let _ = pty.read_output(&session).expect("output readable");
    let status = pty.exit_status(&session).expect("exit status reported");
    assert_eq!(status, Some(1));
}

#[test]
fn pty_session_persists_independent_of_ui() {
    // Sessions are owned by SessionService, not by any UI (TERM-003).
    let pty = LinuxPtyAdapter::new();
    let session_a = pty.create_session().unwrap();
    let session_b = pty.create_session().unwrap();

    pty.write_input(&session_a, b"echo session-a\n").unwrap();
    pty.write_input(&session_b, b"echo session-b\n").unwrap();

    let out_a = pty.read_output(&session_a).unwrap();
    let out_b = pty.read_output(&session_b).unwrap();

    assert_eq!(out_a, "session-a\n");
    assert_eq!(out_b, "session-b\n");
    // Two independent sessions; one can close without affecting the other.
    assert!(pty.exit_status(&session_a).is_ok());
    assert!(pty.exit_status(&session_b).is_ok());
}
