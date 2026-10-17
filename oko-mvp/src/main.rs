//! Host entry point for the ỌKỌ slice-2 MVP (web-based desktop).
//!
//! `oko-host` is now an IPC server. The desktop UI (HTML/CSS/JavaScript, under
//! `ui/`) connects over stdio IPC and calls the capability-checked host API.
//! The core (domain/application) is unchanged from slice 1 and still imports
//! no UI, browser, or graphics code (SRD §4.2).
//!
//! The browser engine is behind `BrowserEnginePort`; the MVP ships a dev
//! `MockBrowserEngineAdapter` that can be swapped for a supported Chromium
//! embedding (Blink/V8/Skia) without touching domain/application.

use oko_core::adapters::{
    linux_pty::LinuxPtyAdapter,
    linux_process::LinuxProcessAdapter,
};
use oko_core::application::session_service::SessionService;
use oko_core::browser::adapters::mock::MockBrowserEngineAdapter;
use oko_core::browser::ports::engine::BrowserEnginePort;
use oko_core::ipc::server::IpcServer;
use oko_core::ports::terminal::TerminalPort;

fn main() {
    // --- Adapters (platform implementations behind ports) ----------------
    let process: Box<dyn oko_core::ports::process::ProcessPort> =
        Box::new(LinuxProcessAdapter::new());
    let terminal: Box<dyn TerminalPort> = Box::new(LinuxPtyAdapter::new());
    let browser: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());

    // --- Core services ---------------------------------------------------
    let sessions = SessionService::new();

    // --- IPC host --------------------------------------------------------
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut server = IpcServer::new(process, terminal, sessions, browser, &workspace_root);

    println!("oko-host v0.2.0 (slice 2: web-based desktop)");
    println!("ipc: stdio (typed, versioned, validated messages)");
    println!("engine: mock/dev (swappable for Blink via Chromium embedding)");
    println!("workspace: {}", workspace_root.display());

    if let Err(e) = server.run() {
        eprintln!("ipc server stopped: {}", e);
        std::process::exit(1);
    }

    println!("oko-host stopped.");
    std::process::exit(0);
}
