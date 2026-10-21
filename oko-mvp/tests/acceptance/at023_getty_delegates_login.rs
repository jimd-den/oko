//! Acceptance test AT-023: getty reads a username and hands auth to login.
//!
//! A real PTY is used because stdin and stdout are the same console there, as
//! they are on a virtual terminal. The stand-in login proves the handoff; the
//! password path belongs to the system's `login`, not to this binary.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;

#[test]
fn getty_passes_a_validated_username_to_login() {
    let dir = std::env::temp_dir().join(format!("oko-getty-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let login = dir.join("fake-login");
    fs::write(&login, "#!/bin/sh\nprintf 'LOGIN %s %s\\n' \"$1\" \"$2\"\n").unwrap();
    let mut perms = fs::metadata(&login).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&login, perms).unwrap();

    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("open pty");
    let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_oko-getty"));
    command.args(["--login", login.to_str().unwrap()]);
    let mut child = pair.slave.spawn_command(command).expect("spawn getty");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("reader");
    let mut writer = pair.master.take_writer().expect("writer");
    writer.write_all(b"alice\n").expect("write username");
    writer.flush().unwrap();
    drop(writer);

    let mut transcript = String::new();
    reader.read_to_string(&mut transcript).expect("read pty");
    let status = child.wait().expect("wait for getty");
    let _ = fs::remove_dir_all(&dir);

    assert!(status.success(), "transcript was {:?}", transcript);
    assert!(
        transcript.contains("LOGIN -p alice"),
        "transcript was {:?}",
        transcript
    );
}
