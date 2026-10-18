//! Application: command service execution flow.
//!
//! Exercises parse → validate → handle → present with real adapters owned
//! inline in each test (avoiding dangling borrows from a helper function).

use oko_core::adapters::{linux_process::LinuxProcessAdapter, linux_pty::LinuxPtyAdapter};
use oko_core::application::command_service::{CommandContext, CommandService};
use oko_core::application::session_service::SessionService;
use oko_core::domain::capability::{Capability, CapabilityPolicy};
use oko_core::domain::command::{Command, CommandResult};

mod builtin_commands;

use self::builtin_commands::register_basic;

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
            _ => {
                return Ok(oko_core::domain::command::CommandResult::Error(
                    "missing".into(),
                ))
            }
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
