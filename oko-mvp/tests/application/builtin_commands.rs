//! The built-in command table the command service flow tests drive.
//!
//! Fixture, not a behaviour: it wires handlers for the built-in commands
//! (help, status, app list/launch/stop), so it changes when the built-in
//! command set changes - a different reason to change than the flow tests.
//! It is not tested directly; every flow test drives it.

use oko_core::application::command_service::CommandService;
use oko_core::domain::command::Command;

pub fn register_basic(svc: &mut CommandService) {
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
            args: vec!["-c".into(), "echo ok".to_string()],
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
