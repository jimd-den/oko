//! oko-session: the unprivileged ỌKỌ console on a bare TTY.
//!
//! This is the program a getty replacement starts after authentication, and
//! the program a user can set as their shell while the login path is being
//! built. It needs no display server, no browser and no GPU: it reads lines
//! from the terminal and drives the same core as `oko-host`.
//!
//! Deliberately line-oriented for now. A full-screen TUI is a presenter on top
//! of the same core; this proves the session and the command path first.

use oko_core::domain::config::Config;
use oko_core::host;
use oko_core::ipc::json::{self, object, JsonValue};
use oko_core::ipc::message::Message;
use std::io::{BufRead, Write};
use std::path::PathBuf;

const USAGE: &str = "usage: oko-session [--engine <name>] [--workspace <path>]";

struct Options {
    engine: String,
    workspace: PathBuf,
}

fn main() {
    let options = match parse_args(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("oko-session: {}", message);
            eprintln!("{}", USAGE);
            std::process::exit(64);
        }
    };
    if let Err(error) = run(&options) {
        eprintln!("oko-session: {}", error);
        std::process::exit(1);
    }
}

fn run(options: &Options) -> Result<(), String> {
    let mut core = host::core(&options.workspace, &options.engine);
    core.start();
    println!("ỌKỌ console");
    println!("engine: {} · workspace: {}", options.engine, options.workspace.display());
    println!("type help for commands, quit to leave");
    repl(&mut core)?;
    core.stop();
    Ok(())
}

fn repl(core: &mut oko_core::ipc::server::Core) -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    loop {
        print!("oko> ");
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        if input.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            println!();
            return Ok(());
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "quit" || line == "exit" {
            return Ok(());
        }
        if line == "help" {
            println!("commands: help, status, engine, apps, quit");
            println!("anything else runs in the shell session");
            continue;
        }
        println!("{}", ask(core, line)?);
    }
}

fn ask(core: &mut oko_core::ipc::server::Core, text: &str) -> Result<String, String> {
    let payload = object(vec![("text".into(), JsonValue::String(text.into()))]);
    let reply = core.handle(&Message::request("1", "computer.ask", payload));
    if let Some(error) = reply.error {
        return Ok(format!("error: {}", error));
    }
    Ok(json::get(&reply.payload, "reply")
        .and_then(json::as_str)
        .unwrap_or("no reply")
        .to_string())
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut engine = Config::default().browser.engine;
    let mut workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--engine" => {
                engine = args.get(i + 1).cloned().ok_or("--engine needs a value")?;
                i += 2;
            }
            "--workspace" => {
                workspace = PathBuf::from(args.get(i + 1).ok_or("--workspace needs a value")?);
                i += 2;
            }
            other => return Err(format!("unknown argument: {}", other)),
        }
    }
    Ok(Options { engine, workspace })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_engine_defaults_to_the_profile() {
        assert_eq!(
            parse_args(&args(&[])).unwrap().engine,
            Config::default().browser.engine
        );
    }

    #[test]
    fn an_engine_and_workspace_are_parsed() {
        let options = parse_args(&args(&["--engine", "mock", "--workspace", "/tmp"])).unwrap();
        assert_eq!(options.engine, "mock");
        assert_eq!(options.workspace, PathBuf::from("/tmp"));
    }

    #[test]
    fn an_unknown_argument_is_refused() {
        assert!(parse_args(&args(&["--frobnicate"])).is_err());
    }
}
