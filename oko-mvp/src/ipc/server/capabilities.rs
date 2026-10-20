//! Which capability each host route requires (SRD §8.3, SEC-002).
//!
//! The host enforces this on every request: a declaration in a UI document, a
//! manifest entry, or a friendly caller grants nothing. The table is the whole
//! permission surface in one place, so adding a route without deciding its
//! capability is visible in review rather than silent.

use crate::domain::capability::Capability;

/// The capabilities `route` requires. Routes that need none return an empty
/// list; dispatch refuses an unknown route before the policy is consulted.
pub fn required_for(route: &str) -> Vec<Capability> {
    match route {
        "host.terminal.create"
        | "host.terminal.read"
        | "host.terminal.write"
        | "host.terminal.close" => vec![Capability::TerminalCreate],
        "host.app.launch" | "host.app.stop" => vec![Capability::ProcessSpawn],
        "host.app.create" => vec![Capability::FilesystemWrite],
        "host.fs.read" => vec![Capability::FilesystemRead],
        "host.fs.write" => vec![Capability::FilesystemWrite],
        "host.fs.list" => vec![Capability::FilesystemRead],
        "host.browser.open" => vec![Capability::BrowserNavigate],
        // The computer answers with the state of the machine and can run a
        // command, so it needs what a terminal needs.
        "computer.ask" => vec![Capability::TerminalCreate],
        // Opening a web pane is a navigation, so it needs the navigation
        // capability even though the host's own engine is not involved.
        "ui.pane.open" => vec![Capability::BrowserNavigate],
        "ui.mode.desktop" => vec![Capability::SystemConfigure],
        "render.demo" | "render.stats" | "render.capture" => vec![Capability::GraphicsCreate],
        _ => vec![],
    }
}

/// A refusal message that names what was missing.
pub fn denial(route: &str, required: &[Capability]) -> String {
    let names: Vec<String> = required.iter().map(|c| format!("{:?}", c)).collect();
    format!("permission denied: {} requires {}", route, names.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_needs_nothing() {
        assert!(required_for("host.status").is_empty());
    }

    #[test]
    fn terminal_create_needs_terminal_create() {
        assert_eq!(
            required_for("host.terminal.create"),
            vec![Capability::TerminalCreate]
        );
    }

    #[test]
    fn changing_the_display_mode_needs_system_configure() {
        assert_eq!(
            required_for("ui.mode.desktop"),
            vec![Capability::SystemConfigure]
        );
    }

    #[test]
    fn a_denial_names_the_route_and_the_capability() {
        let message = denial("host.terminal.create", &[Capability::TerminalCreate]);
        assert!(message.contains("permission denied"));
        assert!(message.contains("TerminalCreate"));
    }
}
