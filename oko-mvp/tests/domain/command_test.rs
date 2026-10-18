//! Red-green TDD tests for domain command parsing and validation.
//!
//! Cycle 1: parse_command — one assertion per test, small composable function.
use oko_core::domain::capability::{Capability, CapabilityPolicy};
use oko_core::domain::command::{parse_command, validate_command, Command};

#[test]
fn parse_help() {
    assert_eq!(parse_command("help"), Ok(Command::Help));
}

#[test]
fn parse_status() {
    assert_eq!(parse_command("status"), Ok(Command::Status));
}

#[test]
fn parse_app_list() {
    assert_eq!(parse_command("app list"), Ok(Command::AppList));
}

#[test]
fn parse_app_launch_with_id() {
    assert_eq!(
        parse_command("app launch terminal"),
        Ok(Command::AppLaunch {
            id: "terminal".into()
        })
    );
}

#[test]
fn parse_app_stop_with_id() {
    assert_eq!(
        parse_command("app stop terminal"),
        Ok(Command::AppStop {
            id: "terminal".into()
        })
    );
}

#[test]
fn parse_browser_open() {
    assert_eq!(
        parse_command("browser open https://example.com"),
        Ok(Command::BrowserOpen {
            url: "https://example.com".into()
        })
    );
}

#[test]
fn parse_render_demo() {
    assert_eq!(parse_command("render demo"), Ok(Command::RenderDemo));
}

#[test]
fn parse_exit() {
    assert_eq!(parse_command("exit"), Ok(Command::Exit));
    assert_eq!(parse_command("quit"), Ok(Command::Exit));
}

#[test]
fn parse_unknown_command() {
    assert!(parse_command("frobnicate").is_err());
}

#[test]
fn parse_empty_input() {
    assert!(parse_command("").is_err());
    assert!(parse_command("   ").is_err());
}

// Cycle 2: validate_command — capability enforcement.
#[test]
fn validate_app_launch_allowed_when_policy_grants_spawn() {
    let policy = CapabilityPolicy::new().with(Capability::ProcessSpawn);
    let cmd = Command::AppLaunch {
        id: "terminal".into(),
    };
    assert!(validate_command(&cmd, &policy).is_ok());
}

#[test]
fn validate_app_launch_denied_without_spawn() {
    let policy = CapabilityPolicy::new(); // empty: nothing granted
    let cmd = Command::AppLaunch {
        id: "terminal".into(),
    };
    assert!(validate_command(&cmd, &policy).is_err());
}

#[test]
fn validate_browser_navigate_denied_without_browser_cap() {
    let policy = CapabilityPolicy::new().with(Capability::ProcessSpawn);
    let cmd = Command::BrowserOpen {
        url: "https://example.com".into(),
    };
    assert!(validate_command(&cmd, &policy).is_err());
}

#[test]
fn validate_help_needs_no_capability() {
    let policy = CapabilityPolicy::new(); // empty
    assert!(validate_command(&Command::Help, &policy).is_ok());
}
