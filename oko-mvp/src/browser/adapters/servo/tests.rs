use super::*;
use crate::browser::ports::engine::BrowserEnginePort;
use std::fs;

/// A stand-in engine: it answers `--version` and otherwise waits to be stopped,
/// which is all this adapter needs from a real binary to exercise its paths.
fn fake_engine() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("oko-fake-servo-{}", std::process::id()));
    fs::write(
        &path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'Version: Servo fake-test'; exit 0; fi\nsleep 30\n",
    )
    .unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[test]
fn servo_identity_unlinked() {
    let engine = ServoBrowserEngineAdapter::new();
    assert_eq!(engine.engine_name(), "servo");
    assert!(engine.is_experimental());
    assert!(engine.capabilities().supports_js);
    assert!(!engine.capabilities().supports_downloads);
}

#[test]
fn servo_unlinked_reports_unavailable() {
    let engine = ServoBrowserEngineAdapter::new();
    assert!(engine.initialize().is_err());
    assert!(engine
        .create_session(crate::browser::ports::engine::BrowserConfig {
            profile: "default".into(),
            url: None,
            off_the_record: false,
        })
        .is_err());
}

#[test]
fn servo_crash_is_isolated() {
    let engine = ServoBrowserEngineAdapter::new();
    engine.simulate_crash("servo renderer crashed");
    assert!(engine.initialize().is_err());
    assert_eq!(
        engine.last_crash_reason(),
        Some("servo renderer crashed".into())
    );
    engine.clear_crash();
    assert_eq!(engine.last_crash_reason(), None);
}

#[test]
fn one_engine_per_instance() {
    let a = ServoBrowserEngineAdapter::new();
    let b = ServoBrowserEngineAdapter::new();
    assert_eq!(a.engine_name(), "servo");
    assert_eq!(b.engine_name(), "servo");
    // No shared internal state between instances (SRD §2.2).
}

#[test]
fn servo_is_experimentally_marked() {
    // SRD ENG-009: experimental engines must be visibly marked.
    let engine = ServoBrowserEngineAdapter::new();
    assert!(engine.is_experimental());
    assert!(engine.capabilities().is_experimental);
}
