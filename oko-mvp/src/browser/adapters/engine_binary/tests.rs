//! The `[servo]` lock record: what a pinned release entry reads, and what it
//! refuses to accept. Pure parsing over a temp file; no network, no engine.

use std::fs;
use std::path::{Path, PathBuf};

use super::{extract, launch, verify};
use super::{parse_servo_entry, ArchiveKind};

/// A throwaway workspace, empty and unique to one test.
fn workspace(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oko-engine-lock-{}", tag));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_lock(dir: &Path, body: &str) -> PathBuf {
    let lock = dir.join("toolchain").join("versions.lock");
    fs::create_dir_all(lock.parent().unwrap()).unwrap();
    fs::write(&lock, body).unwrap();
    lock
}

#[test]
fn reads_the_pinned_release_from_the_servo_section() {
    let tmp = workspace("reads");
    let lock = write_lock(
        &tmp,
        "[servo]\n\
         component = \"Servo\"\n\
         upstream_project = \"Servo\"\n\
         source_url = \"https://example.com/servo.tar.gz\"\n\
         pinned_revision = \"v0.7.0\"\n\
         source_checksum_sha256 = \"0123\"\n\
         archive_kind = \"tar.gz\"\n",
    );

    let entry = parse_servo_entry(&lock).unwrap();

    assert_eq!(entry.name, "servo");
    assert_eq!(entry.url, "https://example.com/servo.tar.gz");
    assert_eq!(entry.sha256, "0123");
    assert_eq!(entry.archive, ArchiveKind::TarGz);
}

#[test]
fn ignores_the_other_sections() {
    let tmp = workspace("other-sections");
    let lock = write_lock(
        &tmp,
        "[other_engine]\n\
         source_url = \"https://example.com/other.tar.xz\"\n\
         source_checksum_sha256 = \"ffff\"\n\
         archive_kind = \"tar.xz\"\n\
         \n\
         [servo]\n\
         source_url = \"https://example.com/servo.tar.gz\"\n\
         source_checksum_sha256 = \"0123\"\n\
         archive_kind = \"tar.gz\"\n",
    );

    let entry = parse_servo_entry(&lock).unwrap();

    assert_eq!(entry.url, "https://example.com/servo.tar.gz");
    assert_eq!(entry.sha256, "0123");
}

#[test]
fn rejects_an_unset_checksum() {
    let tmp = workspace("unset-checksum");
    let lock = write_lock(
        &tmp,
        "[servo]\n\
         source_url = \"https://example.com/servo.tar.gz\"\n\
         archive_kind = \"tar.gz\"\n",
    );

    assert!(parse_servo_entry(&lock).is_err());
}

#[test]
fn rejects_an_unknown_archive_kind() {
    let tmp = workspace("unknown-kind");
    let lock = write_lock(
        &tmp,
        "[servo]\n\
         source_url = \"https://example.com/servo.zip\"\n\
         source_checksum_sha256 = \"0123\"\n\
         archive_kind = \"zip\"\n",
    );

    assert!(parse_servo_entry(&lock).is_err());
}

#[test]
fn archive_file_name_and_tar_mode_match_the_container() {
    assert_eq!(ArchiveKind::TarGz.file_name("servo"), "servo.tar.gz");
    assert_eq!(ArchiveKind::TarXz.file_name("servo"), "servo.tar.xz");
    assert_eq!(ArchiveKind::TarGz.tar_mode(), "-xzf");
    assert_eq!(ArchiveKind::TarXz.tar_mode(), "-xJf");
}

#[test]
fn a_known_digest_verifies_and_a_wrong_one_does_not() {
    let tmp = workspace("verify");
    let file = tmp.join("archive.bin");
    fs::write(&file, b"hello world").unwrap();
    // SHA-256("hello world"), the same vector the digest module pins.
    let digest = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

    assert!(verify::verify(&file, digest).is_ok());
    assert!(verify::verify(&file, &digest.to_uppercase()).is_ok());
    assert!(verify::verify(&file, "deadbeef").is_err());
}

#[test]
fn an_unset_checksum_is_recognised() {
    assert!(verify::is_unset(""));
    assert!(verify::is_unset("  "));
    assert!(verify::is_unset("PLACEHOLDER-PENDING-FIRST-FETCH"));
    assert!(!verify::is_unset("0123"));
}

#[test]
fn a_missing_archive_is_an_error_not_a_panic() {
    let tmp = workspace("missing-archive");
    assert!(verify::verify(&tmp.join("nothing.bin"), "0123").is_err());
    assert!(extract::unpack(
        &tmp.join("nothing.tar.gz"),
        &tmp.join("out"),
        ArchiveKind::TarGz
    )
    .is_err());
}

#[test]
fn the_render_argv_is_the_headless_shape_servo_accepts() {
    let job = launch::RenderJob::new(
        "http://127.0.0.1:8099/",
        Path::new("/tmp/shot.png"),
        "1280x800",
    );
    let argv = launch::render_argv(Path::new("/engines/servo/servoshell"), &job);

    assert_eq!(argv[0], "/engines/servo/servoshell");
    assert!(argv.contains(&"-z".to_string()), "headless flag");
    assert!(argv.contains(&"-x".to_string()), "exit flag");
    assert!(argv.contains(&"/tmp/shot.png".to_string()), "output path");
    assert!(argv.contains(&"--screen-size=1280x800".to_string()));
    assert_eq!(argv.last().unwrap(), "http://127.0.0.1:8099/");
}

#[test]
fn a_render_that_never_finishes_is_stopped_by_the_timeout() {
    let tmp = workspace("timeout");
    let binary = tmp.join("servoshell");
    fs::write(&binary, "#!/bin/sh\nsleep 5\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&binary).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&binary, perms).unwrap();

    let job = launch::RenderJob::new("about:blank", &tmp.join("out.bmp"), "8x8");
    let mut process = launch::spawn_render(&binary, &job).unwrap();
    let result = process.wait_for_exit_timeout(std::time::Duration::from_millis(50));
    assert!(result.is_err(), "the fake engine should not finish");
}

#[test]
fn the_session_argv_names_the_binary_then_the_url() {
    let argv = launch::session_argv(
        Path::new("/engines/servo/servoshell"),
        "http://127.0.0.1:8099/",
    );
    assert_eq!(
        argv,
        vec![
            "/engines/servo/servoshell".to_string(),
            "http://127.0.0.1:8099/".to_string()
        ]
    );
}
