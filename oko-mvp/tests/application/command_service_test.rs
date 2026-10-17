//! Application: command service execution flow.
//!
//! Exercises parse → validate → handle → present with real adapters owned
//! inline in each test (avoiding dangling borrows from a helper function).

use oko_core::application::command_service::{CommandContext, CommandService};
use oko_core::application::session_service::SessionService;
use oko_core::domain::capability::{Capability, CapabilityPolicy};
use oko_core::domain::command::{Command, CommandResult};
use oko_core::adapters::{
    linux_pty::LinuxPtyAdapter,
    linux_process::LinuxProcessAdapter,
};

fn full_policy() -> CapabilityPolicy {
    CapabilityPolicy::new()
        .with(Capability::ProcessSpawn)
        .with(Capability::BrowserNavigate)
        .with(Capability::GraphicsCreate)
        .with(Capability::FilesystemRead)
        .with(Capability::SystemConfigure)
}

fn make_ctx<'a>(
    process: &'a LinuxProcessAdapter,
    terminal: &'a LinuxPtyAdapter,
    policy: &'a CapabilityPolicy,
    sessions: &'a mut SessionService,
) -> CommandContext<'a> {
    CommandContext {
        policy,
        process,
        terminal,
        sessions,
    }
}

fn register_basic(svc: &mut CommandService) {
    svc.register("help", |_cmd, _ctx| {
        Ok(oko_core::domain::command::CommandResult::Help(
            "commands: help".into(),
        ))
    });
    svc.register("status", |_cmd, _ctx| {
        Ok(oko_core::domain::command::CommandResult::Ok(
            "core: running | terminal: available | ui: deferred".into(),
        ))
    });
    svc.register("app list", |_cmd, ctx| {
        let sessions = ctx.sessions.list();
        if sessions.is_empty() {
            return Ok(oko_core::domain::command::CommandResult::Ok(
                "no sessions".into(),
            ));
        }
        Ok(oko_core::domain::command::CommandResult::Ok(
            sessions
                .iter()
                .map(|s| format!("{:?}", s.id.0))
                .collect::<Vec<_>>()
                .join("\n"),
        ))
    });
    svc.register("app launch", |cmd, ctx| {
        let id = match cmd {
            Command::AppLaunch { id } => id.clone(),
            _ => {
                return Ok(oko_core::domain::command::CommandResult::Error(
                    "missing".into(),
                ))
            }
        };
        let req = oko_core::ports::process::SpawnRequest {
            executable: "sh".into(),
            args: vec!["-c".into(), format!("echo ok")],
        };
        let handle = ctx.process.spawn(req).unwrap();
        let _ = ctx.process.stdout(handle).unwrap();
        let session_id = oko_core::domain::session::SessionId(format!("app-{}", id));
        ctx.sessions
            .create_terminal(ctx.terminal, session_id.clone())
            .unwrap();
        Ok(oko_core::domain::command::CommandResult::Ok(format!(
            "launched {}",
            id
        )))
    });
    svc.register("app stop", |cmd, ctx| {
        let id = match cmd {
            Command::AppStop { id } => id.clone(),
            _ => {
                return Ok(oko_core::domain::command::CommandResult::Error(
                    "missing".into(),
                ))
            }
        };
        let session_id = oko_core::domain::session::SessionId(format!("app-{}", id));
        ctx.sessions.stop(&session_id);
        Ok(oko_core::domain::command::CommandResult::Ok(format!(
            "stopped {}",
            id
        )))
    });
}

#[test]
fn execute_help_returns_help_text() {
    let mut svc = CommandService::new();
    register_basic(&mut svc);
    let process = LinuxProcessAdapter::new();
    let terminal = LinuxPtyAdapter::new();
    let policy = full_policy();
    let mut sessions = SessionService::new();
    let mut ctx = make_ctx(&process, &terminal, &policy, &mut sessions);
    let result = svc.execute("help", &mut ctx);
    assert!(result.message().contains("help"));
}

#[test]
fn execute_status_returns_running() {
    let mut svc = CommandService::new();
    register_basic(&mut svc);
    let process = LinuxProcessAdapter::new();
    let terminal = LinuxPtyAdapter::new();
    let policy = full_policy();
    let mut sessions = SessionService::new();
    let mut ctx = make_ctx(&process, &terminal, &policy, &mut sessions);
    let result = svc.execute("status", &mut ctx);
    assert_eq!(
        result.message(),
        "core: running | terminal: available | ui: deferred"
    );
}

#[test]
fn execute_app_launch_then_list_then_stop() {
    let mut svc = CommandService::new();
    register_basic(&mut svc);
    let process = LinuxProcessAdapter::new();
    let terminal = LinuxPtyAdapter::new();
    let policy = full_policy();
    let mut sessions = SessionService::new();
    let mut ctx = make_ctx(&process, &terminal, &policy, &mut sessions);

    let r = svc.execute("app list", &mut ctx);
    assert_eq!(r.message(), "no sessions");

    let r = svc.execute("app launch terminal", &mut ctx);
    assert_eq!(r.message(), "launched terminal");

    let r = svc.execute("app list", &mut ctx);
    assert!(r.message().contains("app-terminal"));

    let r = svc.execute("app stop terminal", &mut ctx);
    assert_eq!(r.message(), "stopped terminal");
}

#[test]
fn execute_unknown_command_errors() {
    let mut svc = CommandService::new();
    register_basic(&mut svc);
    let process = LinuxProcessAdapter::new();
    let terminal = LinuxPtyAdapter::new();
    let policy = full_policy();
    let mut sessions = SessionService::new();
    let mut ctx = make_ctx(&process, &terminal, &policy, &mut sessions);
    let result = svc.execute("frobnicate", &mut ctx);
    assert!(matches!(result, CommandResult::Error(_)));
    assert!(result.message().contains("unknown command"));
}

#[test]
fn execute_denied_without_capability() {
    let mut svc = CommandService::new();
    register_basic(&mut svc);
    let process = LinuxProcessAdapter::new();
    let terminal = LinuxPtyAdapter::new();
    let empty_policy = CapabilityPolicy::new();
    let mut sessions = SessionService::new();
    let mut ctx = make_ctx(&process, &terminal, &empty_policy, &mut sessions);
    let result = svc.execute("app launch terminal", &mut ctx);
    assert!(matches!(result, CommandResult::Error(_)));
    assert!(result.message().contains("permission denied"));
}

#[test]
fn execute_ui_mode_and_browser_engine() {
    let mut svc = CommandService::new();
    register_basic(&mut svc);
    svc.register("ui mode", |cmd, _ctx| {
        let mode = match cmd {
            Command::UiMode { mode } => mode.clone(),
            _ => return Ok(oko_core::domain::command::CommandResult::Error("missing".into())),
        };
        Ok(oko_core::domain::command::CommandResult::Ok(format!(
            "ui mode set to {}",
            mode
        )))
    });
    svc.register("browser engine", |_cmd, _ctx| {
        Ok(oko_core::domain::command::CommandResult::Ok(
            "primary: blink/v8/skia".into(),
        ))
    });
    let process = LinuxProcessAdapter::new();
    let terminal = LinuxPtyAdapter::new();
    let policy = full_policy();
    let mut sessions = SessionService::new();
    let mut ctx = make_ctx(&process, &terminal, &policy, &mut sessions);
    assert_eq!(
        svc.execute("ui mode desktop", &mut ctx).message(),
        "ui mode set to desktop"
    );
    assert_eq!(
        svc.execute("browser engine", &mut ctx).message(),
        "primary: blink/v8/skia"
    );
}
