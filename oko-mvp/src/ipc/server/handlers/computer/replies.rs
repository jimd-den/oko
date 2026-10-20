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
    // Mark the row before writing: the shell may echo the command before this
    // line is reached, and a mark taken after the write can already be below
    // the answer (the test that caught this saw an empty slice).
    let before_cursor = server.terminal.cursor_line(&pty).unwrap_or(0);
    server
        .terminal
        .write_input(&pty, format!("{}\n", command).as_bytes())
        .map_err(|e| format!("{:?}", e))?;
    let output = server
        .terminal
        .read_output(&pty)
        .map_err(|e| format!("{:?}", e))?;
    let after_cursor = server.terminal.cursor_line(&pty).unwrap_or(0);
    let lines: Vec<&str> = output.lines().collect();
    // Lines the command newly drew: from the cursor's old row to its new row.
    let slice = if after_cursor >= before_cursor {
        &lines[before_cursor..after_cursor.min(lines.len())]
    } else {
        &lines[..]
    };
    // Drop the echoed command line and any prompt line; keep the rest.
    let mut answer: Vec<&str> = slice
        .iter()
        .copied()
        .filter(|l| !l.trim_start().starts_with("violindocgfx@") && !l.contains(command))
        .collect();
    // trim trailing empties
    while answer.last().map(|s| s.trim().is_empty()).unwrap_or(false) {
        answer.pop();
    }
    let trimmed = answer.join("\n").trim().to_string();
    if trimmed.is_empty() {
        return Ok("Command completed with no output.".into());
    }
    Ok(trimmed)
}
