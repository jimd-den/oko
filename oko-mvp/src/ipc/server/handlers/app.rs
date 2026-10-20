//! Handler: host.app.list / host.app.launch / host.app.stop.
//! Small composable functions, single responsibility each.

use crate::domain::session::SessionId;
use crate::ipc::json::object;
use crate::ipc::server::Core;
use crate::ports::process::SpawnRequest;

pub fn handle_list(server: &Core) -> Result<crate::ipc::json::JsonValue, String> {
    let sessions_list: Vec<String> = server
        .sessions
        .list()
        .iter()
        .map(|s| format!("{:?}", s.id.0))
        .collect();
    let apps = list_apps(server);
    Ok(object(vec![
        (
            "sessions".into(),
            arr(sessions_list.iter().map(|s| str(s)).collect()),
        ),
        ("apps".into(), arr(apps)),
    ]))
}

/// The HTML applications served from `ui_root/apps`, newest surface first.
/// A page is an app because it is there; there is no separate registry to
/// drift out of step with the directory.
fn list_apps(server: &Core) -> Vec<crate::ipc::json::JsonValue> {
    let dir = server.ui_root.join("apps");
    let mut apps = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("html") {
                continue;
            }
            let id = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            apps.push(object(vec![
                ("id".into(), str(&id)),
                ("name".into(), str(&id)),
                ("url".into(), str(&format!("/apps/{}.html", id))),
            ]));
        }
    }
    apps.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
    apps
}

pub fn handle_launch(
    server: &mut Core,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let id = payload_string(payload, "id")?;
    let req = SpawnRequest {
        executable: "sh".into(),
        args: vec!["-c".into(), format!("echo 'app {} running'", id)],
    };
    let _handle = server.process.spawn(req).map_err(|e| format!("{:?}", e))?;
    let session_id = SessionId(format!("app-{}", id));
    server
        .sessions
        .create_terminal(server.terminal.as_ref(), session_id.clone())
        .map_err(|e| format!("{:?}", e))?;
    let url = format!("/apps/{}.html", id);
    let exists = server.ui_root.join("apps").join(format!("{}.html", id)).is_file();
    if exists {
        Ok(object(vec![("id".into(), str(&id)), ("url".into(), str(&url))]))
    } else {
        Ok(object(vec![("id".into(), str(&id))]))
    }
}

pub fn handle_stop(
    server: &mut Core,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let id = payload_string(payload, "id")?;
    let session_id = SessionId(format!("app-{}", id));
    server.sessions.stop(&session_id);
    Ok(object(vec![("id".into(), str(&id))]))
}

fn str(v: &str) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::String(v.into())
}

fn arr(items: Vec<crate::ipc::json::JsonValue>) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::Array(items)
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
    fn handle_list_returns_empty_when_no_sessions() {
        let server = test_server();
        let result = handle_list(&server).unwrap();
        let _sessions = crate::ipc::json::get(&result, "sessions");
        assert!(_sessions.is_some());
    }

    #[test]
    fn handle_launch_creates_session() {
        let mut server = test_server();
        let payload = crate::ipc::json::object(vec![("id".into(), str("calc"))]);
        let result = handle_launch(&mut server, &payload).unwrap();
        let id = crate::ipc::json::get(&result, "id")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(id, "calc");
    }

    #[test]
    fn handle_stop_removes_session() {
        let mut server = test_server();
        let _ = handle_launch(
            &mut server,
            &crate::ipc::json::object(vec![("id".into(), str("calc"))]),
        );
        let result = handle_stop(
            &mut server,
            &crate::ipc::json::object(vec![("id".into(), str("calc"))]),
        )
        .unwrap();
        let id = crate::ipc::json::get(&result, "id")
            .and_then(crate::ipc::json::as_str)
            .unwrap();
        assert_eq!(id, "calc");
    }

    fn str(v: &str) -> crate::ipc::json::JsonValue {
        crate::ipc::json::JsonValue::String(v.into())
    }

    fn test_server() -> Core {
        use crate::application::session_service::SessionService;
        use crate::browser::adapters::mock::MockBrowserEngineAdapter;
        use crate::browser::ports::engine::BrowserEnginePort;
        use crate::domain::capability::{Capability, CapabilityPolicy};
        use crate::ports::process::ProcessPort;
        use crate::ports::terminal::TerminalPort;

        let process: Box<dyn ProcessPort> =
            Box::new(crate::adapters::linux_process::LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> =
            Box::new(crate::adapters::portable_pty::PortablePtyAdapter::new());
        let sessions = SessionService::new();
        let policy = CapabilityPolicy::new().with(Capability::ProcessSpawn);
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());

        Core::new(
            process,
            terminal,
            sessions,
            policy,
            browser,
        )
    }

    #[test]
    fn handle_list_names_the_html_apps_in_the_ui_root() {
        let dir = std::env::temp_dir().join(format!("oko-apps-{}", std::process::id()));
        let apps = dir.join("apps");
        let _ = std::fs::create_dir_all(&apps);
        std::fs::write(apps.join("readout.html"), "x").unwrap();
        let server = test_server().with_ui_root(dir);
        let result = handle_list(&server).unwrap();
        let list = match crate::ipc::json::get(&result, "apps") {
            Some(crate::ipc::json::JsonValue::Array(a)) => a,
            _ => panic!("no apps array"),
        };
        assert_eq!(list.len(), 1);
        assert_eq!(
            crate::ipc::json::get(&list[0], "url").and_then(crate::ipc::json::as_str),
            Some("/apps/readout.html")
        );
    }
}
