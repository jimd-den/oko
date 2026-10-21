//! Acceptance test AT-021: the console session starts with no display server.
//!
//! The bare-metal promise is that the core and a terminal session do not need
//! X11, Wayland, a GPU or Servo to run. This drives the real `oko-session`
//! binary with both display variables removed and checks that it answers.

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn the_session_starts_without_a_display_and_answers_help_and_status() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_oko-session"))
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("oko-session must start");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        write!(stdin, "help\nstatus\nquit\n").expect("write script");
    }

    let output = child.wait_with_output().expect("oko-session must exit");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "exit: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        stdout,
        stderr
    );
    assert!(stdout.contains("ỌKỌ console"), "stdout was {}", stdout);
    assert!(stdout.contains("commands:"), "stdout was {}", stdout);
    assert!(stdout.contains("Core is running"), "stdout was {}", stdout);
}
