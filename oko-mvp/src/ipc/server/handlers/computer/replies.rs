//! How the computer words its answers.
//!
//! Kept apart from the handler so the reading of a request and the phrasing of
//! its answer can change for different reasons: the grammar lives in the
//! domain, the flow in `mod.rs`, and the words here.

use crate::domain::session::SessionId;
use crate::ipc::server::Core;

pub(super) fn status_reply(server: &Core) -> String {
    let terminal = if server.sessions.list().is_empty() {
        "no sessions open"
    } else {
        "sessions open"
    };
    let engine = match &server.engine_unavailable {
        Some(_) => "the browser engine is unavailable",
        None => "the browser engine is ready",
    };
    let ui = if server.desktop_mode {
        "desktop"
    } else {
        "terminal"
    };
    format!(
        "All systems report ready. Core is running, {}; {}; display mode {}.",
        terminal, engine, ui
    )
}

pub(super) fn engine_reply(server: &Core) -> String {
    match &server.engine_unavailable {
        Some(reason) => format!(
            "The browser engine is {}. {}",
            server.browser.engine_name(),
            reason
        ),
        None => format!(
            "The browser engine is {} {}, {}.",
            server.browser.engine_name(),
            server.browser.engine_version(),
            if server.browser.is_experimental() {
                "experimental"
            } else {
                "a production candidate"
            }
        ),
    }
}

pub(super) fn open_reply(server: &mut Core, url: &str) -> Result<String, String> {
    if let Some(reason) = &server.engine_unavailable {
        return Ok(format!("I cannot open {}: {}.", url, reason));
    }
    let config = crate::browser::ports::engine::BrowserConfig {
        profile: "default".into(),
        url: Some(url.to_string()),
        off_the_record: false,
    };
    server
        .browser
        .create_session(config)
        .map_err(|e| format!("{:?}", e))?;
    Ok(format!("Opening {}.", url))
}

pub(super) fn run_reply(
    server: &mut Core,
    id: &SessionId,
    command: &str,
) -> Result<String, String> {
    let pty = server.sessions.pty(id).ok_or("session not found")?.clone();
    server
        .terminal
        .write_input(&pty, format!("{}\n", command).as_bytes())
        .map_err(|e| format!("{:?}", e))?;
    let output = server
        .terminal
        .read_output(&pty)
        .map_err(|e| format!("{:?}", e))?;
    let trimmed = output.trim_end();
    if trimmed.is_empty() {
        Ok("Command completed with no output.".into())
    } else {
        Ok(trimmed.to_string())
    }
}
