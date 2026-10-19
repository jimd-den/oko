//! The adapter's behaviour against a real shell.
//!
//! These tests start real processes, so they wait for a condition with a
//! deadline rather than reading once: a test that reads once fails on a busy
//! machine, and a flaky test is worse than no test.

use super::*;
use std::time::{Duration, Instant};

/// Wait for `check` to hold, up to a deadline.
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
fn a_real_shell_runs_a_command_and_reports_its_output() {
    let adapter = PortablePtyAdapter::new();
    let session = adapter.create_session().expect("a PTY session");

    adapter
        .write_input(&session, b"echo oko-hello\n")
        .expect("write");
    let mut seen = String::new();
    assert!(
        eventually(|| {
            seen.push_str(&adapter.read_output(&session).unwrap_or_default());
            seen.contains("oko-hello")
        }),
        "shell never echoed the command; saw {:?}",
        seen
    );
}

#[test]
fn a_real_shell_reports_the_exit_status_of_the_session() {
    let adapter = PortablePtyAdapter::new();
    let session = adapter.create_session().expect("a PTY session");

    adapter.write_input(&session, b"exit 7\n").expect("write");
    let mut status = None;
    assert!(
        eventually(|| {
            status = adapter.exit_status(&session).unwrap_or(None);
            status.is_some()
        }),
        "shell never exited; last status {:?}",
        status
    );
    assert_eq!(
        status,
        Some(7),
        "the shell's own exit code must be reported"
    );
}

#[test]
fn two_sessions_are_independent() {
    let adapter = PortablePtyAdapter::new();
    let first = adapter.create_session().unwrap();
    let second = adapter.create_session().unwrap();
    adapter.write_input(&first, b"echo first\n").unwrap();
    adapter.write_input(&second, b"echo second\n").unwrap();

    let mut out_first = String::new();
    let mut out_second = String::new();
    assert!(eventually(|| {
        out_first.push_str(&adapter.read_output(&first).unwrap_or_default());
        out_second.push_str(&adapter.read_output(&second).unwrap_or_default());
        out_first.contains("first") && out_second.contains("second")
    }));
    assert!(
        !out_first.contains("second"),
        "one session's output must not appear in another"
    );
}

#[test]
fn the_screen_applies_control_sequences_instead_of_showing_them() {
    let adapter = PortablePtyAdapter::new();
    let session = adapter.create_session().expect("a PTY session");

    // `oko-done` is produced by the shell, not typed into it, so the string can
    // only reach the caller as output. The escape sequence moves the cursor, so
    // a rendered screen has it applied and a raw buffer still carries it.
    adapter
        .write_input(&session, b"printf \"\\033[2;5Hoko-$(printf done)\\n\"\n")
        .expect("write");
    let mut screen = String::new();
    assert!(
        eventually(|| {
            screen = adapter.read_output(&session).unwrap_or_default();
            screen.contains("oko-done")
        }),
        "the screen never showed the command's output; last read {:?}",
        screen
    );
    assert!(
        !screen.contains('\u{1b}'),
        "control sequences reached the caller instead of the grid: {:?}",
        screen
    );
}

#[test]
fn an_unknown_session_is_an_error_not_a_panic() {
    let adapter = PortablePtyAdapter::new();
    let stranger = PtySession(SessionId("never-created".into()));
    assert!(adapter.read_output(&stranger).is_err());
    assert!(adapter.write_input(&stranger, b"x").is_err());
}

#[test]
fn a_session_can_be_resized() {
    let adapter = PortablePtyAdapter::new();
    let session = adapter.create_session().unwrap();
    assert!(adapter.resize(&session, 120, 40).is_ok());
}
