//! Handler: render.demo / render.stats / render.capture.
//! Small composable functions, single responsibility each.

use crate::graphics::software::Framebuffer;
use crate::ipc::json::object;
use crate::ipc::server::IpcServer;
use crate::ipc::transport::IpcTransport;
use std::fs;

pub fn handle_demo<T: IpcTransport>(
    _server: &mut IpcServer<T>,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let message = payload_string(payload, "message").unwrap_or_else(|_| "oko".into());
    let mut fb = Framebuffer::new(640, 480);
    fb.render_demo(&message);
    let ppm = fb.to_ppm();
    let path =
        payload_string(payload, "path").unwrap_or_else(|_| "/tmp/oko-render-demo.ppm".into());
    fs::write(&path, &ppm).map_err(|e| format!("write demo: {}", e))?;
    Ok(object(vec![
        (
            "output".into(),
            str(&format!(
                "rendered {}x{} software demo -> {}",
                fb.width, fb.height, path
            )),
        ),
        ("path".into(), str(&path)),
        ("backend".into(), str("software")),
    ]))
}

pub fn handle_stats<T: IpcTransport>(
    _server: &IpcServer<T>,
) -> Result<crate::ipc::json::JsonValue, String> {
    Ok(object(vec![
        (
            "stats".into(),
            str("software renderer: 640x480 demo framebuffer"),
        ),
        ("backend".into(), str("software")),
        ("accelerated".into(), bool(false)),
    ]))
}

pub fn handle_capture<T: IpcTransport>(
    _server: &mut IpcServer<T>,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let path = payload_string(payload, "path")?;
    let mut fb = Framebuffer::new(1280, 720);
    fb.render_demo("oko render capture");
    let ppm = fb.to_ppm();
    fs::write(&path, &ppm).map_err(|e| format!("write capture: {}", e))?;
    Ok(object(vec![
        ("path".into(), str(&path)),
        ("backend".into(), str("software")),
    ]))
}

fn str(v: &str) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::String(v.into())
}

fn bool(b: bool) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::Bool(b)
}

fn payload_string(payload: &crate::ipc::json::JsonValue, key: &str) -> Result<String, String> {
    match crate::ipc::json::get(payload, key).and_then(crate::ipc::json::as_str) {
        Some(s) => Ok(s.into()),
        None => Err(format!("missing payload field: {}", key)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_demo_returns_software_backend() {
        let mut server = test_server();
        let payload = crate::ipc::json::object(vec![]);
        let result = handle_demo(&mut server, &payload).unwrap();
        let backend = crate::ipc::json::get(&result, "backend")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(backend, "software");
    }

    #[test]
    fn handle_stats_returns_software() {
        let server = test_server();
        let result = handle_stats(&server).unwrap();
        let backend = crate::ipc::json::get(&result, "backend")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(backend, "software");
    }

    #[test]
    fn handle_capture_writes_file() {
        let mut server = test_server();
        // Per-process name: a fixed path in the shared temp directory is a race
        // between two test processes that write and delete the same file, which
        // is how this test flakes (AGENTS.md §2.5, determinism).
        let out = std::env::temp_dir().join(format!(
            "oko-render-capture-handler-test-{}.ppm",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&out);
        let payload = crate::ipc::json::object(vec![("path".into(), str(&out.to_string_lossy()))]);
        let result = handle_capture(&mut server, &payload);
        assert!(result.is_ok());
        assert!(out.exists());
        let _ = std::fs::remove_file(&out);
    }

    fn str(v: &str) -> crate::ipc::json::JsonValue {
        crate::ipc::json::JsonValue::String(v.into())
    }

    fn test_server() -> IpcServer<crate::ipc::transport::StdioTransport> {
        use crate::application::session_service::SessionService;
        use crate::browser::adapters::mock::MockBrowserEngineAdapter;
        use crate::browser::ports::engine::BrowserEnginePort;
        use crate::domain::capability::{Capability, CapabilityPolicy};
        use crate::ports::process::ProcessPort;
        use crate::ports::terminal::TerminalPort;
        use std::env::temp_dir;

        let process: Box<dyn ProcessPort> =
            Box::new(crate::adapters::linux_process::LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> =
            Box::new(crate::adapters::linux_pty::LinuxPtyAdapter::new());
        let sessions = SessionService::new();
        let policy = CapabilityPolicy::new().with(Capability::GraphicsCreate);
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let root = temp_dir().join("oko-render-test");
        let _ = std::fs::create_dir_all(root.join("toolchain"));

        IpcServer::new_stdio(process, terminal, sessions, policy, browser, &root)
    }
}
