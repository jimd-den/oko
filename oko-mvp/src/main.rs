//! Host entry point for the ỌKỌ MVP.
//!
//! `oko-host` owns the core: sessions, processes, capability policy, the
//! browser engine and the terminal. It is reachable two ways, over the same
//! typed, versioned messages:
//!
//!   --transport stdio   one message per line, for automation and the terminal
//!   --transport http    the desktop shell, served on loopback for a browser
//!                       engine to render and drive (SRD UI-001, UI-008)
//!
//! The engine is chosen by name: `--engine <name>`, or the profile's
//! `[browser] engine` when the flag is absent (SRD ENG-002). An engine this
//! build cannot serve is reported as unavailable and the core keeps working
//! (SRD ENG-008, §4.2); it is never replaced by another.

mod cli;

use cli::Options;
use oko_core::adapters::{linux_process::LinuxProcessAdapter, portable_pty::PortablePtyAdapter};
use oko_core::application::session_service::SessionService;
use oko_core::browser::adapters::{blink_install::BlinkInstaller, select};
use oko_core::domain::capability::{Capability, CapabilityPolicy};
use oko_core::ipc::http::HttpUiServer;
use oko_core::ipc::server::{Core, IpcServer};
use oko_core::ports::terminal::TerminalPort;
use std::path::{Path, PathBuf};

fn main() {
    let options = match cli::parse(std::env::args().skip(1).collect()) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("oko-host: {}", message);
            eprintln!("{}", cli::USAGE);
            std::process::exit(cli::USAGE_EXIT);
        }
    };

    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if options.install_engine {
        std::process::exit(cli::install_engine(&workspace_root, &options.engine));
    }

    let core = Core::new(
        Box::new(LinuxProcessAdapter::new()),
        Box::new(PortablePtyAdapter::new()) as Box<dyn TerminalPort>,
        SessionService::new(),
        policy(),
        select::engine_for(&options.engine, &workspace_root),
        BlinkInstaller::new(&workspace_root),
    );

    println!("oko-host v0.3.0 (slice 3: terminal and web panes in one desktop)");
    println!("engine: {}", options.engine);
    println!(
        "engines in this build: {}",
        select::engine_names().join(", ")
    );
    println!("workspace: {}", workspace_root.display());

    let outcome = match options.transport.as_str() {
        "http" => serve_ui(core, &options, &workspace_root),
        _ => {
            println!("ipc: stdio (one typed, versioned message per line)");
            IpcServer::new_stdio(core).run()
        }
    };

    if let Err(e) = outcome {
        eprintln!("oko-host stopped: {}", e);
        std::process::exit(1);
    }
    println!("oko-host stopped.");
}

/// Serve the desktop shell on loopback for a browser engine to render.
fn serve_ui(core: Core, options: &Options, workspace_root: &Path) -> Result<(), String> {
    let core = core.with_ui_root(options.ui_root.clone());
    let server = HttpUiServer::bind_on(core, &options.ui_root, &options.bind, options.port)?;
    let url = server.url();
    let page_url = format!("{}{}", url, options.page.trim_start_matches('/'));
    println!("ui: {}", url);
    println!("page: {}", page_url);

    let mut engine = if options.open_engine {
        Some(cli::launch_engine(
            &options.engine,
            &page_url,
            workspace_root,
        )?)
    } else {
        println!("open {} in any engine, or pass --open-engine", page_url);
        None
    };

    let outcome = server.serve();
    if let Some(process) = engine.as_mut() {
        let _ = process.stop();
    }
    outcome
}

/// The capabilities a desktop session holds (SRD §8.3).
fn policy() -> CapabilityPolicy {
    CapabilityPolicy::new()
        .with(Capability::ProcessSpawn)
        .with(Capability::BrowserNavigate)
        .with(Capability::GraphicsCreate)
        .with(Capability::FilesystemRead)
        .with(Capability::FilesystemWrite)
        .with(Capability::SystemConfigure)
        .with(Capability::TerminalCreate)
}
