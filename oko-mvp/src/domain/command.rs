//! Domain: command definitions and parsing.
//!
//! Pure functions, no platform dependencies. Small and composable so each can
//! be covered by a single red-green TDD cycle.

use crate::domain::capability::{Capability, CapabilityPolicy};

/// A stable command identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandId(pub String);

/// Domain commands. Kept small; the host API maps UI/automation requests to
/// these (SRD §7.3 command model).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Status,
    AppList,
    AppLaunch { id: String },
    AppStop { id: String },
    UiMode { mode: String },
    UiSplit,
    BrowserOpen { url: String },
    BrowserEngine,
    RenderDemo,
    RenderStats,
    RenderCapture { path: String },
    ImageOpen { path: String },
    SystemLogs,
    SystemRecovery,
    Exit,
    Unknown { input: String },
}

/// Errors produced while parsing a command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    UnknownCommand(String),
}

/// Parse a raw input line into a `Command`. Pure and composable.
pub fn parse_command(input: &str) -> Result<Command, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }
    let mut parts = trimmed.split_whitespace();
    let head = parts.next().unwrap();
    let rest: String = parts.collect::<Vec<_>>().join(" ");

    match head {
        "help" => Ok(Command::Help),
        "status" => Ok(Command::Status),
        "app" => match rest.trim() {
            "list" => Ok(Command::AppList),
            s if s.starts_with("launch ") => {
                let id = s["launch ".len()..].trim().to_string();
                if id.is_empty() {
                    Err(ParseError::UnknownCommand("app launch <id>".to_string()))
                } else {
                    Ok(Command::AppLaunch { id })
                }
            }
            s if s.starts_with("stop ") => {
                let id = s["stop ".len()..].trim().to_string();
                if id.is_empty() {
                    Err(ParseError::UnknownCommand("app stop <id>".to_string()))
                } else {
                    Ok(Command::AppStop { id })
                }
            }
            _ => Err(ParseError::UnknownCommand(format!("app {}", rest))),
        },
        "ui" => match rest.trim() {
            s if s.starts_with("mode ") => {
                let mode = s["mode ".len()..].trim().to_string();
                Ok(Command::UiMode { mode })
            }
            "split" => Ok(Command::UiSplit),
            _ => Err(ParseError::UnknownCommand(format!("ui {}", rest))),
        },
        "browser" => match rest.trim() {
            s if s.starts_with("open ") => {
                let url = s["open ".len()..].trim().to_string();
                Ok(Command::BrowserOpen { url })
            }
            "engine" => Ok(Command::BrowserEngine),
            _ => Err(ParseError::UnknownCommand(format!("browser {}", rest))),
        },
        "render" => match rest.trim() {
            "demo" => Ok(Command::RenderDemo),
            "stats" => Ok(Command::RenderStats),
            s if s.starts_with("capture ") => {
                let path = s["capture ".len()..].trim().to_string();
                Ok(Command::RenderCapture { path })
            }
            _ => Err(ParseError::UnknownCommand(format!("render {}", rest))),
        },
        "image" => match rest.trim() {
            s if s.starts_with("open ") => {
                let path = s["open ".len()..].trim().to_string();
                Ok(Command::ImageOpen { path })
            }
            _ => Err(ParseError::UnknownCommand(format!("image {}", rest))),
        },
        "system" => match rest.trim() {
            "logs" => Ok(Command::SystemLogs),
            "recovery" => Ok(Command::SystemRecovery),
            _ => Err(ParseError::UnknownCommand(format!("system {}", rest))),
        },
        "exit" | "quit" => Ok(Command::Exit),
        _ => Err(ParseError::UnknownCommand(head.to_string())),
    }
}

/// Capabilities required by a command (used by validation).
fn capabilities_required(cmd: &Command) -> Vec<Capability> {
    match cmd {
        Command::AppLaunch { .. } | Command::AppStop { .. } => {
            vec![Capability::ProcessSpawn]
        }
        Command::BrowserOpen { .. } => vec![Capability::BrowserNavigate],
        Command::RenderDemo | Command::RenderStats | Command::RenderCapture { .. } => {
            vec![Capability::GraphicsCreate]
        }
        Command::ImageOpen { .. } => vec![Capability::FilesystemRead],
        Command::SystemRecovery => vec![Capability::SystemConfigure],
        _ => vec![],
    }
}

/// Validate a command against the capability policy. Pure and composable.
pub fn validate_command(cmd: &Command, policy: &CapabilityPolicy) -> Result<(), ValidationError> {
    let required = capabilities_required(cmd);
    if policy.allows(&required) {
        Ok(())
    } else {
        Err(ValidationError::MissingCapability(required))
    }
}

/// Validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    MissingCapability(Vec<Capability>),
    InvalidArgument(String),
}

/// Result of executing a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    Ok(String),
    Help(String),
    Error(String),
}

impl CommandResult {
    pub fn message(&self) -> &str {
        match self {
            CommandResult::Ok(m) | CommandResult::Help(m) | CommandResult::Error(m) => m,
        }
    }
}

impl std::fmt::Display for CommandResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandResult::Ok(m) => write!(f, "{}", m),
            CommandResult::Help(m) => write!(f, "{}", m),
            CommandResult::Error(m) => write!(f, "error: {}", m),
        }
    }
}
