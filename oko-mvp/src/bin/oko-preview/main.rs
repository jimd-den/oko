//! oko-preview: render a URL with Servo and show it on a TTY or a display server.
//!
//! Servo renders offscreen to a BMP; the BMP becomes a `Frame`. With no socket,
//! `TerminalPresenter` draws the frame as truecolor half blocks. With
//! `--socket`, the frame is pushed as a surface to `oko-display`, which owns the
//! compositor and the actual output.
//!
//! Still a preview: no input, no live navigation and no scrolling. It proves
//! the offscreen renderer, the frame format and the display contract fit.

use oko_core::browser::adapters::engine_binary::{self, RenderJob};
use oko_core::display::protocol::{ClientMessage, ServerMessage};
use oko_core::display::surface::SurfaceId;
use oko_core::display::wire::{read_server, write_client};
use oko_core::display::{decode_bmp, Frame, Presenter, TerminalPresenter};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

const USAGE: &str = "usage: oko-preview <url> [cols] [rows] [--socket <path>]";

struct Options {
    url: String,
    cols: u32,
    rows: u32,
    socket: Option<PathBuf>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--help") {
        println!("{}", USAGE);
        return;
    }
    let options = match parse_args(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("oko-preview: {}", message);
            eprintln!("{}", USAGE);
            std::process::exit(64);
        }
    };
    if let Err(error) = run(&options) {
        eprintln!("oko-preview: {}", error);
        std::process::exit(1);
    }
}

fn run(options: &Options) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|e| format!("cwd: {}", e))?;
    let binary = engine_binary::locate(&root, "servo").ok_or_else(|| {
        "no servo binary under third_party/engines/servo; run `oko-host --install-engine servo`"
            .to_string()
    })?;
    let output = std::env::temp_dir().join(format!("oko-preview-{}.bmp", std::process::id()));
    let width = options.cols.saturating_mul(8);
    let height = options.rows.saturating_mul(16);
    let job = RenderJob::new(&options.url, &output, &format!("{}x{}", width, height));
    let mut process = engine_binary::spawn_render(&binary, &job)?;
    let exit = process.wait_for_exit_timeout(std::time::Duration::from_secs(30))?;

    let bytes = std::fs::read(&output).map_err(|e| {
        format!(
            "read {}: {} (engine exit: {:?}, screen {}x{})",
            output.display(),
            e,
            exit,
            width,
            height
        )
    })?;
    let frame = decode_bmp(&bytes).map_err(|e| format!("decode bmp: {:?}", e))?;
    let _ = std::fs::remove_file(&output);

    match &options.socket {
        Some(path) => push_surface(path, frame),
        None => {
            let mut presenter = TerminalPresenter::new(std::io::stdout(), options.cols, options.rows);
            presenter
                .present(&frame)
                .map_err(|e| format!("present: {:?}", e))
        }
    }
}

fn push_surface(path: &PathBuf, frame: Frame) -> Result<(), String> {
    let mut stream = UnixStream::connect(path)
        .map_err(|e| format!("connect {}: {}", path.display(), e))?;
    let mut reader = stream.try_clone().map_err(|e| e.to_string())?;
    let id = SurfaceId(1);
    write_client(
        &mut stream,
        &ClientMessage::Create {
            id,
            width: frame.width,
            height: frame.height,
        },
    )?;
    expect_created(&mut reader)?;
    write_client(&mut stream, &ClientMessage::Commit { id, frame })?;
    expect_committed(&mut reader)
}

fn expect_created(reader: &mut UnixStream) -> Result<(), String> {
    match read_server(reader)? {
        Some(ServerMessage::Created { .. }) => Ok(()),
        Some(ServerMessage::Error { reason }) => Err(reason),
        other => Err(format!("expected created, got {:?}", other)),
    }
}

fn expect_committed(reader: &mut UnixStream) -> Result<(), String> {
    match read_server(reader)? {
        Some(ServerMessage::Committed { .. }) => Ok(()),
        Some(ServerMessage::Error { reason }) => Err(reason),
        other => Err(format!("expected committed, got {:?}", other)),
    }
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut positional = Vec::new();
    let mut socket = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--socket" => {
                socket = Some(PathBuf::from(
                    args.get(i + 1).ok_or("--socket needs a value")?,
                ));
                i += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown argument: {}", other));
            }
            other => {
                positional.push(other.to_string());
                i += 1;
            }
        }
    }
    let url = positional
        .first()
        .ok_or_else(|| format!("missing url\n{}", USAGE))?
        .clone();
    let cols = parse_dimension(positional.get(1), 80)?;
    let rows = parse_dimension(positional.get(2), 24)?;
    Ok(Options {
        url,
        cols,
        rows,
        socket,
    })
}

fn parse_dimension(value: Option<&String>, default: u32) -> Result<u32, String> {
    match value {
        None => Ok(default),
        Some(text) => match text.parse::<u32>() {
            Ok(0) | Err(_) => Err(format!("dimension must be a positive number: {}", text)),
            Ok(n) => Ok(n),
        },
    }
}

#[cfg(test)]
mod tests;
