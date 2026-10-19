use super::*;

fn root() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("oko-fs-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn server() -> (Core, std::path::PathBuf) {
    use crate::application::session_service::SessionService;
    use crate::browser::adapters::blink_install::BlinkInstaller;
    use crate::browser::adapters::mock::MockBrowserEngineAdapter;
    use crate::browser::ports::engine::BrowserEnginePort;
    use crate::domain::capability::{Capability, CapabilityPolicy};
    use crate::ports::process::ProcessPort;
    use crate::ports::terminal::TerminalPort;
    let dir = root();
    let process: Box<dyn ProcessPort> =
        Box::new(crate::adapters::linux_process::LinuxProcessAdapter::new());
    let terminal: Box<dyn TerminalPort> =
        Box::new(crate::adapters::portable_pty::PortablePtyAdapter::new());
    let sessions = SessionService::new();
    let policy = CapabilityPolicy::new()
        .with(Capability::FilesystemRead)
        .with(Capability::FilesystemWrite);
    let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
    let core = Core::new(process, terminal, sessions, policy, browser, BlinkInstaller::new(&dir))
        .with_workspace_root(dir.clone());
    (core, dir)
}

#[test]
fn lists_files_in_the_workspace() {
    let (core, dir) = server();
    std::fs::write(dir.join("a.txt"), "x").unwrap();
    let payload = crate::ipc::json::object(vec![("path".into(), str("."))]);
    let reply = handle_list(&core, &payload).unwrap();
    let names: Vec<String> = crate::ipc::json::get(&reply, "entries")
        .and_then(|v| if let crate::ipc::json::JsonValue::Array(a) = v { Some(a) } else { None })
        .unwrap()
        .iter()
        .filter_map(|e| crate::ipc::json::get(e, "name").and_then(crate::ipc::json::as_str).map(String::from))
        .collect();
    assert!(names.contains(&"a.txt".to_string()), "names were {:?}", names);
}

#[test]
fn reads_a_file_inside_the_workspace() {
    let (core, dir) = server();
    std::fs::write(dir.join("note.txt"), "hello").unwrap();
    let payload = crate::ipc::json::object(vec![("path".into(), str("note.txt"))]);
    let reply = handle_read(&core, &payload).unwrap();
    assert_eq!(crate::ipc::json::get(&reply, "text").and_then(crate::ipc::json::as_str), Some("hello"));
}

#[test]
fn refuses_a_path_outside_the_workspace() {
    let (core, _dir) = server();
    let payload = crate::ipc::json::object(vec![("path".into(), str("../../etc/passwd"))]);
    assert!(handle_read(&core, &payload).is_err());
}

#[test]
fn writes_a_file_inside_the_workspace() {
    let (core, dir) = server();
    let payload = crate::ipc::json::object(vec![
        ("path".into(), str("out.txt")),
        ("text".into(), str("data")),
    ]);
    handle_write(&core, &payload).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("out.txt")).unwrap(), "data");
}

fn str(v: &str) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::String(v.into())
}
