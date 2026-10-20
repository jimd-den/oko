//! oko-preview: render a URL with Servo and draw it on the current terminal.
//!
//! This is the first end-to-end display path: no display server, no browser
//! window, no GPU requirement. Servo renders offscreen to a BMP; the BMP
//! becomes a `Frame`; `TerminalPresenter` draws it as truecolor half blocks.
//!
//! It is deliberately a preview, not a session: there is no input, no live
//! navigation and no scrolling. The display server's surface protocol is the
//! next step; this proves the presenter and the offscreen renderer fit together.

use oko_core::browser::adapters::engine_binary::{self, RenderJob};
use oko_core::display::{decode_bmp, Presenter, TerminalPresenter};

const USAGE: &str = "usage: oko-preview <url> [cols] [rows]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--help") {
        println!("{}", USAGE);
        return;
    }
    if let Err(error) = run(&args) {
        eprintln!("oko-preview: {}", error);
        std::process::exit(1);
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let (url, cols, rows) = parse_args(args)?;
    let root = std::env::current_dir().map_err(|e| format!("cwd: {}", e))?;
    let binary = engine_binary::locate(&root, "servo").ok_or_else(|| {
        "no servo binary under third_party/engines/servo; run `oko-host --install-engine servo`"
            .to_string()
    })?;
    let output = std::env::temp_dir().join(format!("oko-preview-{}.bmp", std::process::id()));
    let width = cols.saturating_mul(8);
    let height = rows.saturating_mul(16);
    let job = RenderJob::new(&url, &output, &format!("{}x{}", width, height));
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
    let mut presenter = TerminalPresenter::new(std::io::stdout(), cols, rows);
    presenter
        .present(&frame)
        .map_err(|e| format!("present: {:?}", e))?;
    let _ = std::fs::remove_file(&output);
    Ok(())
}

fn parse_args(args: &[String]) -> Result<(String, u32, u32), String> {
    let url = args
        .first()
        .ok_or_else(|| format!("missing url\n{}", USAGE))?
        .clone();
    let cols = parse_dimension(args.get(1), 80)?;
    let rows = parse_dimension(args.get(2), 24)?;
    Ok((url, cols, rows))
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
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn defaults_to_80_by_24() {
        assert_eq!(
            parse_args(&args(&["https://example.com"])).unwrap(),
            ("https://example.com".into(), 80, 24)
        );
    }

    #[test]
    fn a_missing_url_is_refused() {
        assert!(parse_args(&args(&[])).is_err());
    }

    #[test]
    fn a_zero_dimension_is_refused() {
        assert!(parse_args(&args(&["x", "0"])).is_err());
    }
}
