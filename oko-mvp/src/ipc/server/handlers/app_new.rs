//! Handler: host.app.create — scaffold a new application page.
//!
//! A person should be able to make a computer of their own from the terminal,
//! without an editor handshake: pick a template, name it, and the host writes a
//! page from `ui_root/templates/<template>.html` into `ui_root/apps/`. The page
//! is served live, so the next `app open <name>` shows it.
//!
//! Deliberately not here: editing and launching. Those are `host.fs.*` and
//! `host.app.launch`; this file only turns a name and a template into a file.

use crate::ipc::json::object;
use crate::ipc::server::Core;

pub fn handle_create(
    server: &Core,
    payload: &crate::ipc::json::JsonValue,
) -> Result<crate::ipc::json::JsonValue, String> {
    let name = payload_string(payload, "name")?;
    let template = payload_string(payload, "template").unwrap_or_else(|_| "blank".into());
    if !valid_name(&name) {
        return Err(format!(
            "app name '{}' must be letters, digits, '-' or '_'",
            name
        ));
    }
    let source = server
        .ui_root
        .join("templates")
        .join(format!("{}.html", template));
    let text = std::fs::read_to_string(&source)
        .map_err(|_| format!("no template '{}'", template))?;
    let apps = server.ui_root.join("apps");
    std::fs::create_dir_all(&apps).map_err(|e| format!("create apps dir: {}", e))?;
    let target = apps.join(format!("{}.html", name));
    if target.exists() {
        return Err(format!("app '{}' already exists", name));
    }
    std::fs::write(&target, text).map_err(|e| format!("write app: {}", e))?;
    Ok(object(vec![
        ("id".into(), str(&name)),
        ("url".into(), str(&format!("/apps/{}.html", name))),
    ]))
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn payload_string(
    payload: &crate::ipc::json::JsonValue,
    key: &str,
) -> Result<String, String> {
    match crate::ipc::json::get(payload, key).and_then(crate::ipc::json::as_str) {
        Some(s) => Ok(s.into()),
        None => Err(format!("missing payload field: {}", key)),
    }
}

fn str(v: &str) -> crate::ipc::json::JsonValue {
    crate::ipc::json::JsonValue::String(v.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::session_service::SessionService;
    use crate::browser::adapters::blink_install::BlinkInstaller;
    use crate::browser::adapters::mock::MockBrowserEngineAdapter;
    use crate::browser::ports::engine::BrowserEnginePort;
    use crate::domain::capability::{Capability, CapabilityPolicy};
    use crate::ports::process::ProcessPort;
    use crate::ports::terminal::TerminalPort;

    fn server() -> (Core, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("oko-app-new-{}", std::process::id()));
        let _ = std::fs::create_dir_all(dir.join("templates"));
        let _ = std::fs::create_dir_all(dir.join("apps"));
        std::fs::write(dir.join("templates").join("blank.html"), "PAGE").unwrap();
        let process: Box<dyn ProcessPort> =
            Box::new(crate::adapters::linux_process::LinuxProcessAdapter::new());
        let terminal: Box<dyn TerminalPort> =
            Box::new(crate::adapters::portable_pty::PortablePtyAdapter::new());
        let policy = CapabilityPolicy::new().with(Capability::FilesystemWrite);
        let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
        let core = Core::new(
            process,
            terminal,
            SessionService::new(),
            policy,
            browser,
            BlinkInstaller::new(&dir),
        )
        .with_ui_root(dir.clone());
        (core, dir)
    }

    fn payload(name: &str, template: &str) -> crate::ipc::json::JsonValue {
        crate::ipc::json::object(vec![
            ("name".into(), str(name)),
            ("template".into(), str(template)),
        ])
    }

    #[test]
    fn creates_an_app_from_a_template() {
        let (core, dir) = server();
        let reply = handle_create(&core, &payload("bridge", "blank")).unwrap();
        assert_eq!(
            crate::ipc::json::get(&reply, "url").and_then(crate::ipc::json::as_str),
            Some("/apps/bridge.html")
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("apps").join("bridge.html")).unwrap(),
            "PAGE"
        );
    }

    #[test]
    fn refuses_a_name_that_is_a_path() {
        let (core, _dir) = server();
        assert!(handle_create(&core, &payload("../escape", "blank")).is_err());
    }

    #[test]
    fn refuses_to_overwrite_an_existing_app() {
        let (core, dir) = server();
        std::fs::write(dir.join("apps").join("bridge.html"), "OLD").unwrap();
        assert!(handle_create(&core, &payload("bridge", "blank")).is_err());
    }

    #[test]
    fn names_the_missing_template() {
        let (core, _dir) = server();
        let err = handle_create(&core, &payload("bridge", "nope")).unwrap_err();
        assert!(err.contains("nope"));
    }
}
