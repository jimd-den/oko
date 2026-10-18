//! Application: command registry and execution (SRD §7.3).
//!
//! Each command has a stable identifier, typed arguments, validation rules,
//! structured output, human-readable presentation, error code and exit status.

use crate::application::session_service::SessionService;
use crate::domain::capability::CapabilityPolicy;
use crate::domain::command::{
    parse_command, validate_command, Command, CommandId, CommandResult, ParseError,
    ValidationError,
};
use crate::ports::process::ProcessPort;
use crate::ports::terminal::TerminalPort;

/// Exit status codes for the host (human-readable presentation + exit status).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    Usage = 64,
    Unavailable = 69,
    Software = 70,
    NotPermitted = 77,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// A command handler closure. Small composable functions register here.
/// Receives the parsed command (for arguments like `app launch <id>`) and the
/// shared context (ports, sessions, policy) mutably so handlers can spawn
/// processes and create sessions.
pub type Handler =
    dyn Fn(&Command, &mut CommandContext) -> Result<CommandResult, String> + Send + Sync;

/// Context passed to every handler: ports, session service, capability policy.
pub struct CommandContext<'a> {
    pub policy: &'a CapabilityPolicy,
    pub process: &'a dyn ProcessPort,
    pub terminal: &'a dyn TerminalPort,
    pub sessions: &'a mut SessionService,
}

/// The command service: register handlers and execute typed commands.
pub struct CommandService {
    handlers: std::collections::HashMap<CommandId, Box<Handler>>,
    next_pid: u64,
}

impl Default for CommandService {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandService {
    pub fn new() -> Self {
        Self {
            handlers: std::collections::HashMap::new(),
            next_pid: 1,
        }
    }

    /// Register a handler for a command id.
    pub fn register<F>(&mut self, id: &str, handler: F)
    where
        F: Fn(&Command, &mut CommandContext) -> Result<CommandResult, String>
            + Send
            + Sync
            + 'static,
    {
        self.handlers.insert(CommandId(id.to_string()), Box::new(handler));
    }

    /// Execute a raw input line: parse → validate → handle → present.
    pub fn execute(&self, input: &str, ctx: &mut CommandContext) -> CommandResult {
        let cmd = match parse_command(input) {
            Ok(c) => c,
            Err(ParseError::Empty) => return CommandResult::Ok(String::new()),
            Err(ParseError::UnknownCommand(c)) => {
                return CommandResult::Error(format!("unknown command: {}", c));
            }
        };

        if matches!(cmd, Command::Exit) {
            return CommandResult::Ok("goodbye".into());
        }

        if let Err(ValidationError::MissingCapability(caps)) =
            validate_command(&cmd, ctx.policy)
        {
            let names: Vec<String> = caps.iter().map(|c| format!("{:?}", c)).collect();
            return CommandResult::Error(format!(
                "permission denied: requires {}",
                names.join(", ")
            ));
        }

        let handler = match self.handlers.get(&command_id(&cmd)) {
            Some(h) => h,
            None => return CommandResult::Error(format!("no handler for {:?}", cmd)),
        };

        match handler(&cmd, ctx) {
            Ok(result) => result,
            Err(msg) => CommandResult::Error(msg),
        }
    }

    pub fn next_pid(&mut self) -> u64 {
        let id = self.next_pid;
        self.next_pid += 1;
        id
    }
}

fn command_id(cmd: &Command) -> CommandId {
    match cmd {
        Command::Help => CommandId("help".into()),
        Command::Status => CommandId("status".into()),
        Command::AppList => CommandId("app list".into()),
        Command::AppLaunch { .. } => CommandId("app launch".into()),
        Command::AppStop { .. } => CommandId("app stop".into()),
        Command::UiMode { .. } => CommandId("ui mode".into()),
        Command::UiSplit => CommandId("ui split".into()),
        Command::BrowserOpen { .. } => CommandId("browser open".into()),
        Command::BrowserEngine => CommandId("browser engine".into()),
        Command::RenderDemo => CommandId("render demo".into()),
        Command::RenderStats => CommandId("render stats".into()),
        Command::RenderCapture { .. } => CommandId("render capture".into()),
        Command::ImageOpen { .. } => CommandId("image open".into()),
        Command::SystemLogs => CommandId("system logs".into()),
        Command::SystemRecovery => CommandId("system recovery".into()),
        Command::Exit => CommandId("exit".into()),
        Command::Unknown { input } => CommandId(format!("unknown:{}", input)),
    }
}
