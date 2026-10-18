//! Spawn a prebuilt engine process: the argv for the two real use cases, and
//! a handle that stops it.
//!
//! Deliberately does NOT go through `ports::process::ProcessPort`.
//! `LinuxProcessAdapter::spawn` runs the command to completion with `.output()`
//! and caches the result, and its `kill` is a documented no-op: the port models
//! a command that has already finished. An engine session must stay alive for
//! as long as the browser session does, and a headless render must stay alive
//! until its PNG is written, so the lifetime the port offers is the wrong shape.
//!
//! Only flags the pinned Servo release is known to accept appear here (`-z`,
//! `-o`, `--screen-size`): an invented flag fails at runtime in a way no
//! offline test can catch.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// What one headless render asks the engine for.
#[derive(Debug, Clone)]
pub struct RenderJob {
    /// The page to render.
    pub url: String,
    /// The PNG to write.
    pub output: PathBuf,
    /// The viewport, `<width>x<height>`.
    pub screen_size: String,
}

impl RenderJob {
    pub fn new(url: &str, output: &Path, screen_size: &str) -> Self {
        Self {
            url: url.into(),
            output: output.to_path_buf(),
            screen_size: screen_size.into(),
        }
    }
}

/// How a spawned engine process ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineExit {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

/// A spawned engine process the host can stop.
pub struct EngineProcess {
    child: Child,
}

impl EngineProcess {
    /// The OS process id.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Wait for the engine to exit on its own and report how.
    pub fn wait_for_exit(&mut self) -> Result<EngineExit, String> {
        let status = self.child.wait().map_err(|e| format!("wait: {}", e))?;
        Ok(EngineExit {
            code: status.code(),
            signal: exit_signal(&status),
        })
    }

    /// Stop the engine, then reap it so no zombie is left behind.
    pub fn stop(&mut self) -> Result<(), String> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        Ok(())
    }
}

/// argv for a headless render: `-z -o <png> --screen-size <WxH> <url>`.
pub fn render_argv(binary: &Path, job: &RenderJob) -> Vec<String> {
    vec![
        binary.to_string_lossy().to_string(),
        "-z".into(),
        "-o".into(),
        job.output.to_string_lossy().to_string(),
        format!("--screen-size={}", job.screen_size),
        job.url.clone(),
    ]
}

/// argv for a windowed session on `url`.
///
/// Deliberately no profile or storage flag: the UI keeps its own profile, and
/// this package passes no flag it has not verified against a real engine.
pub fn session_argv(binary: &Path, url: &str) -> Vec<String> {
    vec![binary.to_string_lossy().to_string(), url.to_string()]
}

/// Spawn a headless render and return its handle.
pub fn spawn_render(binary: &Path, job: &RenderJob) -> Result<EngineProcess, String> {
    spawn(&render_argv(binary, job))
}

/// Spawn a windowed session and return its handle.
pub fn spawn_session(binary: &Path, url: &str) -> Result<EngineProcess, String> {
    spawn(&session_argv(binary, url))
}

/// Spawn `argv`; the first element names the program.
fn spawn(argv: &[String]) -> Result<EngineProcess, String> {
    let (program, rest) = argv.split_first().ok_or("empty argv")?;
    let child = Command::new(program)
        .args(rest)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn {}: {}", program, e))?;
    Ok(EngineProcess { child })
}

#[cfg(unix)]
fn exit_signal(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn exit_signal(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}
