//! Domain: what the user asked the computer, and what that means.
//!
//! The Star Trek computer takes a sentence and does the obvious thing. Without
//! natural language understanding, the honest version is a small command
//! grammar plus one rule: anything this module does not recognise is a shell
//! command, because that is what a computer is for. The reply says which
//! reading was taken, so the user is never guessing.
//!
//! Pure: no I/O, no policy, no host. The handler decides what an intent costs.

/// What one request means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// Report the state of the system.
    Status,
    /// Report the browser engine and its state.
    Engine,
    /// Say what this computer is.
    Identify,
    /// Open a URL in a web pane.
    Open(String),
    /// Run this in the shell.
    Run(String),
    /// Nothing was said.
    Nothing,
}

/// Words that mean "report yourself", in the order they are worth checking.
const STATUS_WORDS: [&str; 5] = ["status", "report", "systems", "diagnostics", "health"];

/// Read `text` as an intent.
///
/// Contract: case-insensitive for the words this module knows; a leading
/// "computer" (with any punctuation) is dropped, because that is how one
/// addresses this machine; everything else becomes `Run` unchanged, so the
/// shell sees exactly what was typed.
pub fn interpret(text: &str) -> Intent {
    let spoken = strip_address(text);
    if spoken.is_empty() {
        return Intent::Nothing;
    }
    let lowered = spoken.to_lowercase();

    if STATUS_WORDS.contains(&lowered.as_str()) {
        return Intent::Status;
    }
    if lowered == "engine" || lowered == "browser" {
        return Intent::Engine;
    }
    if matches!(
        lowered.as_str(),
        "who are you" | "identify" | "hello" | "hi"
    ) {
        return Intent::Identify;
    }
    if let Some(url) = lowered.strip_prefix("open ") {
        return Intent::Open(url.trim().to_string());
    }
    if let Some(command) = spoken.strip_prefix("run ") {
        return Intent::Run(command.trim().to_string());
    }
    Intent::Run(spoken.to_string())
}

/// Drop a leading address ("computer", "computer,", "hey computer") and trim.
fn strip_address(text: &str) -> &str {
    let trimmed = text.trim();
    let lowered = trimmed.to_lowercase();
    for prefix in ["hey computer", "computer"] {
        if lowered.starts_with(prefix) {
            let rest = &trimmed[prefix.len()..];
            return rest
                .trim_start_matches(|c: char| c.is_whitespace() || ",:!.".contains(c))
                .trim();
        }
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_status_word_is_a_status_request() {
        assert_eq!(interpret("status"), Intent::Status);
        assert_eq!(interpret("Report"), Intent::Status);
    }

    #[test]
    fn addressing_the_computer_first_is_allowed() {
        assert_eq!(interpret("computer, status"), Intent::Status);
        assert_eq!(interpret("Computer: report"), Intent::Status);
        assert_eq!(interpret("hey computer engine"), Intent::Engine);
    }

    #[test]
    fn an_unrecognised_line_is_a_shell_command() {
        assert_eq!(interpret("ls -la"), Intent::Run("ls -la".into()));
        assert_eq!(interpret("echo hello"), Intent::Run("echo hello".into()));
    }

    #[test]
    fn run_and_open_are_explicit_forms() {
        assert_eq!(interpret("run uname -a"), Intent::Run("uname -a".into()));
        assert_eq!(
            interpret("open /apps/hello.html"),
            Intent::Open("/apps/hello.html".into())
        );
    }

    #[test]
    fn the_command_keeps_its_case() {
        assert_eq!(interpret("echo Hello"), Intent::Run("echo Hello".into()));
    }

    #[test]
    fn silence_is_not_a_command() {
        assert_eq!(interpret("   "), Intent::Nothing);
        assert_eq!(interpret("computer,"), Intent::Nothing);
    }

    #[test]
    fn identity_questions_are_recognised() {
        assert_eq!(interpret("who are you"), Intent::Identify);
    }
}
