//! Command line, and the two engine actions the host can be asked to perform.
//!
//! Kept apart from the composition root in `main.rs`: this file decides *what
//! was asked for*, `main.rs` decides *what that is wired to*. Neither knows how
//! a message is transported.

use oko_core::browser::adapters::engine_binary::{self, EngineProcess, InstallOutcome};
use oko_core::domain::config::Config;
use std::path::{Path, PathBuf};

/// Exit status for a bad command line, matching the command model (SRD §7.3).
pub const USAGE_EXIT: i32 = 64;

pub const USAGE: &str = "usage: oko-host [--engine <name>] [--transport stdio|http] \
[--port <n>] [--bind <addr>] [--page <path>] [--ui <dir>] \
[--install-engine] [--open-engine]";

pub struct Options {
    pub engine: String,
    pub transport: String,
    pub port: u16,
    pub bind: String,
    pub ui_root: PathBuf,
    pub page: String,
    pub install_engine: bool,
    pub open_engine: bool,
}

pub fn parse(args: Vec<String>) -> Result<Options, String> {
    let value_after = |flag: &str| -> Option<&String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
    };

    let engine = match value_after("--engine") {
        Some(name) => name.clone(),
        None => Config::default().browser.engine,
    };
    let transport = value_after("--transport")
        .cloned()
        .unwrap_or_else(|| "stdio".into());
    if transport != "stdio" && transport != "http" {
        return Err(format!("unknown transport '{}'", transport));
    }
    let port = match value_after("--port") {
        Some(value) => value.parse().map_err(|_| format!("bad port '{}'", value))?,
        None => 0,
    };
    let ui_root = value_after("--ui").cloned().unwrap_or_else(|| "ui".into());
    // Loopback by default: publishing the shell publishes the host API.
    let bind = value_after("--bind")
        .cloned()
        .unwrap_or_else(|| "127.0.0.1".into());
    let page = value_after("--page").cloned().unwrap_or_else(|| "/".into());

    Ok(Options {
        engine,
        transport,
        port,
        bind,
        ui_root: PathBuf::from(ui_root),
        page,
        install_engine: args.iter().any(|a| a == "--install-engine"),
        open_engine: args.iter().any(|a| a == "--open-engine"),
    })
}

/// Start the installed engine on the UI URL.
///
/// The host owns the engine process, so the desktop is rendered by the engine
/// this build was configured for rather than by whatever is on the machine.
pub fn launch_engine(
    name: &str,
    url: &str,
    workspace_root: &Path,
) -> Result<EngineProcess, String> {
    let binary = engine_binary::locate(workspace_root, name).ok_or_else(|| {
        format!(
            "no {} binary under third_party/engines/{}; run `oko-host --install-engine {}` first",
            name, name, name
        )
    })?;
    let process = engine_binary::spawn_session(&binary, url)?;
    println!(
        "engine process: {} (pid {})",
        binary.display(),
        process.pid()
    );
    Ok(process)
}

/// `--install-engine`: fetch the pinned prebuilt release, verify it, unpack it.
///
/// Contract: never runs as part of a normal start (SRD DEP-002), refuses when
/// the pinned checksum is unset, and fails loudly on a mismatch (DEP-003).
pub fn install_engine(workspace_root: &Path, name: &str) -> i32 {
    if name != "servo" {
        eprintln!(
            "oko-host: no prebuilt release is pinned for '{}'; toolchain/versions.lock pins servo",
            name
        );
        return USAGE_EXIT;
    }
    let lock = workspace_root.join("toolchain").join("versions.lock");
    let entry = match engine_binary::parse_servo_entry(&lock) {
        Ok(entry) => entry,
        Err(why) => {
            eprintln!("oko-host: {}", why);
            return 1;
        }
    };
    match engine_binary::install(workspace_root, &entry) {
        InstallOutcome::AlreadyInstalled(path) | InstallOutcome::Installed(path) => {
            println!("engine ready: {}", path.display());
            0
        }
        InstallOutcome::Failed(why) => {
            eprintln!("oko-host: install failed: {}", why);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_transport_defaults_to_stdio() {
        let options = parse(args(&[])).unwrap();
        assert_eq!(options.transport, "stdio");
        assert_eq!(options.port, 0);
    }

    #[test]
    fn the_engine_defaults_to_the_profile() {
        assert_eq!(
            parse(args(&[])).unwrap().engine,
            Config::default().browser.engine
        );
        assert_eq!(parse(args(&["--engine", "servo"])).unwrap().engine, "servo");
    }

    #[test]
    fn an_unknown_transport_is_refused() {
        assert!(parse(args(&["--transport", "carrier-pigeon"])).is_err());
    }

    #[test]
    fn a_bad_port_is_refused() {
        assert!(parse(args(&["--port", "eighty"])).is_err());
    }

    #[test]
    fn the_two_engine_actions_are_flags() {
        let options = parse(args(&["--install-engine", "--open-engine"])).unwrap();
        assert!(options.install_engine);
        assert!(options.open_engine);
    }
}
