//! oko-display: a surface display server on the current TTY.
//!
//! Apps connect over a Unix socket and create/commit/destroy surfaces. The
//! server composites them and presents the frame as truecolor ANSI half blocks.
//! No display server, no browser window, no GPU.
//!
//! This is the first live display server, not the full one: one connection at a
//! time, no input pump yet, and a terminal presenter. The surface contract is
//! the same one a framebuffer or browser-canvas presenter will serve.

use oko_core::display::server::{serve_unix, serve_unix_with_input};
use oko_core::display::session::DisplaySession;
use oko_core::display::terminal::TerminalPresenter;
use oko_core::display::TerminalInput;
use std::path::{Path, PathBuf};

const USAGE: &str = "usage: oko-display [--socket <path>] [--width <px>] [--height <px>] [--cols <n>] [--rows <n>]";

struct Options {
    socket: PathBuf,
    width: u32,
    height: u32,
    cols: u32,
    rows: u32,
}

fn main() {
    let options = match parse_args(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("oko-display: {}", message);
            eprintln!("{}", USAGE);
            std::process::exit(64);
        }
    };
    if let Err(error) = run(&options) {
        eprintln!("oko-display: {}", error);
        std::process::exit(1);
    }
}

fn run(options: &Options) -> Result<(), String> {
    prepare_socket(&options.socket)?;
    let _guard = SocketGuard(options.socket.clone());
    let presenter = TerminalPresenter::new(std::io::stdout(), options.cols, options.rows);
    let mut session = DisplaySession::new(options.width, options.height, presenter);
    let input = TerminalInput::spawn().ok();
    eprintln!(
        "oko-display: {} ({}x{}, {}x{} cells), input {}",
        options.socket.display(),
        options.width,
        options.height,
        options.cols,
        options.rows,
        if input.is_some() { "raw tty" } else { "unavailable" }
    );
    match &input {
        Some(input) => serve_unix_with_input(&options.socket, &mut session, input.receiver()),
        None => serve_unix(&options.socket, &mut session),
    }
}

/// Refuse a regular file, replace a stale socket. The path is user-chosen; a
/// server that deletes an arbitrary file because it was named on the command
/// line is a footgun.
fn prepare_socket(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    use std::os::unix::fs::FileTypeExt;
    if path.metadata().map_err(|e| e.to_string())?.file_type().is_socket() {
        std::fs::remove_file(path).map_err(|e| format!("remove stale socket: {}", e))
    } else {
        Err(format!("{} exists and is not a socket", path.display()))
    }
}

struct SocketGuard(PathBuf);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        socket: PathBuf::from("/tmp/oko-display.sock"),
        width: 640,
        height: 384,
        cols: 80,
        rows: 24,
    };
    let mut i = 0;
    while i < args.len() {
        let value = args.get(i + 1).ok_or_else(|| format!("{} needs a value", args[i]))?;
        match args[i].as_str() {
            "--socket" => options.socket = PathBuf::from(value),
            "--width" => options.width = parse_dimension(value)?,
            "--height" => options.height = parse_dimension(value)?,
            "--cols" => options.cols = parse_dimension(value)?,
            "--rows" => options.rows = parse_dimension(value)?,
            other => return Err(format!("unknown argument: {}", other)),
        }
        i += 2;
    }
    Ok(options)
}

fn parse_dimension(value: &str) -> Result<u32, String> {
    match value.parse::<u32>() {
        Ok(0) | Err(_) => Err(format!("dimension must be a positive number: {}", value)),
        Ok(n) => Ok(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn defaults_are_a_640x384_frame_in_80x24_cells() {
        let options = parse_args(&args(&[])).unwrap();
        assert_eq!((options.width, options.height), (640, 384));
        assert_eq!((options.cols, options.rows), (80, 24));
    }

    #[test]
    fn the_socket_and_size_can_be_overridden() {
        let options = parse_args(&args(&[
            "--socket", "/tmp/x.sock", "--width", "320", "--height", "200", "--cols", "40",
            "--rows", "12",
        ]))
        .unwrap();
        assert_eq!(options.socket, PathBuf::from("/tmp/x.sock"));
        assert_eq!((options.width, options.height), (320, 200));
        assert_eq!((options.cols, options.rows), (40, 12));
    }

    #[test]
    fn a_zero_dimension_is_refused() {
        assert!(parse_args(&args(&["--width", "0"])).is_err());
    }
}
