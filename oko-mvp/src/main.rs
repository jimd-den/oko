//! Host entry point for the ỌKỌ MVP (web-based desktop).
//!
//! `oko-host` is an IPC server. The desktop UI (HTML/CSS/JavaScript, under
//! `ui/`) connects over stdio IPC and calls the capability-checked host API.
//! The core (domain/application) imports no UI, browser, or graphics code
//! (SRD §4.2).
//!
//! The browser engine is chosen by name at startup: `--engine <name>`, or the
//! profile's `[browser] engine` when the flag is absent (SRD ENG-002). An
//! engine this build cannot serve is reported as unavailable and the terminal
//! core keeps working (SRD ENG-008, §4.2); it is never replaced by another.

use oko_core::adapters::{linux_pty::LinuxPtyAdapter, linux_process::LinuxProcessAdapter};
use oko_core::application::session_service::SessionService;
use oko_core::browser::adapters::select;
use oko_core::domain::capability::{Capability, CapabilityPolicy};
use oko_core::domain::config::Config;
use oko_core::ipc::server::IpcServer;
use oko_core::ports::terminal::TerminalPort;

/// Exit status for a bad command line, matching the command model (SRD §7.3).
const USAGE_EXIT: i32 = 64;

fn main() {
    let requested = match requested_engine(std::env::args().skip(1).collect()) {
        Ok(name) => name,
        Err(message) => {
            eprintln!("oko-host: {}", message);
            eprintln!("usage: oko-host [--engine <name>]");
            std::process::exit(USAGE_EXIT);
        }
    };
    let browser = select::engine_for(&requested);

    // --- Adapters (platform implementations behind ports) ----------------
    let process: Box<dyn oko_core::ports::process::ProcessPort> =
        Box::new(LinuxProcessAdapter::new());
    let terminal: Box<dyn TerminalPort> = Box::new(LinuxPtyAdapter::new());

    // --- Core services ---------------------------------------------------
    let sessions = SessionService::new();

    // --- Capability policy (SRD §8.3) -----------------------------------
    let policy = CapabilityPolicy::new()
        .with(Capability::ProcessSpawn)
        .with(Capability::BrowserNavigate)
        .with(Capability::GraphicsCreate)
        .with(Capability::FilesystemRead)
        .with(Capability::SystemConfigure)
        .with(Capability::TerminalCreate);

    // --- IPC host --------------------------------------------------------
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut server = IpcServer::new_stdio(
        process, terminal, sessions, policy, browser, &workspace_root,
    );

    println!("oko-host v0.2.0 (slice 2: web-based desktop)");
    println!("ipc: stdio (typed, versioned, validated messages)");
    println!("engine: {}", requested);
    println!("engines in this build: {}", select::engine_names().join(", "));
    println!("workspace: {}", workspace_root.display());
    println!("engine state: ask the host API route host.browser.engine");

    if let Err(e) = server.run() {
        eprintln!("ipc server stopped: {}", e);
        std::process::exit(1);
    }

    println!("oko-host stopped.");
    std::process::exit(0);
}

/// The engine name to serve: `--engine <name>` overrides the profile's
/// `[browser] engine` (SRD ENG-002, §11).
fn requested_engine(args: Vec<String>) -> Result<String, String> {
    match args.iter().position(|arg| arg == "--engine") {
        Some(index) => args
            .get(index + 1)
            .cloned()
            .ok_or_else(|| "--engine needs an engine name".to_string()),
        None => Ok(Config::default().browser.engine),
    }
}
