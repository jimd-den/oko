//! oko-getty: a getty replacement that delegates authentication.
//!
//! It owns the console prompt: it opens the TTY, prints the ỌKỌ banner, reads a
//! username, validates it, and execs the system `login`. `login` performs the
//! password check and starts the user's shell. If that shell is
//! `oko-session`, the user lands in ỌKỌ.
//!
//! Why delegate: `login` is the platform's audited authentication boundary,
//! uses PAM on Linux/FreeBSD and BSD auth on OpenBSD/NetBSD, and already does
//! the TTY ownership, utmp and privilege drop. This binary never sees a
//! password, never links PAM, and never runs setuid.
//!
//! Started by init/systemd as root, like getty. If exec fails, it exits; init
//! must respawn it, and a fallback getty on another VT is part of the install.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const USAGE: &str = "usage: oko-getty [--tty <path>] [--login <path>]";
const DEFAULT_LOGIN: &str = "/bin/login";
const MAX_USERNAME: usize = 32;

fn main() {
    let options = match parse_args(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("oko-getty: {}", message);
            eprintln!("{}", USAGE);
            std::process::exit(64);
        }
    };
    if let Err(error) = run(&options) {
        eprintln!("oko-getty: {}", error);
        std::process::exit(1);
    }
}

struct Options {
    tty: Option<PathBuf>,
    login: PathBuf,
}

fn run(options: &Options) -> Result<(), String> {
    let mut tty = match &options.tty {
        Some(path) => OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| format!("open {}: {}", path.display(), e))?,
        None => return login_on_inherited_stdio(&options.login),
    };

    write!(tty, "ỌKỌ console\nlogin: ").map_err(|e| e.to_string())?;
    tty.flush().map_err(|e| e.to_string())?;
    let user = read_username(&tty)?;
    exec_login(&options.login, &user, &tty)
}

fn login_on_inherited_stdio(login: &Path) -> Result<(), String> {
    print!("ỌKỌ console\nlogin: ");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let user = validate_username(line.trim())?;
    let error = Command::new(login).args(login_args(&user)).exec();
    Err(format!("exec {}: {}", login.display(), error))
}

fn read_username(tty: &std::fs::File) -> Result<String, String> {
    let mut line = String::new();
    let mut reader = BufReader::new(tty.try_clone().map_err(|e| e.to_string())?);
    reader
        .read_line(&mut line)
        .map_err(|e| format!("read username: {}", e))?;
    validate_username(line.trim())
}

fn exec_login(login: &Path, user: &str, tty: &std::fs::File) -> Result<(), String> {
    let input = tty.try_clone().map_err(|e| e.to_string())?;
    let output = tty.try_clone().map_err(|e| e.to_string())?;
    let error = tty.try_clone().map_err(|e| e.to_string())?;
    let result = Command::new(login)
        .args(login_args(user))
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(output))
        .stderr(Stdio::from(error))
        .exec();
    Err(format!("exec {}: {}", login.display(), result))
}

/// Arguments for the system login. `-p` preserves TERM and the rest of the
/// environment that getty is expected to pass through.
fn login_args(user: &str) -> [String; 2] {
    ["-p".into(), user.into()]
}

/// A conservative username: it is passed as one argument, never through a
/// shell, but it is still validated because this is a privilege boundary.
fn validate_username(raw: &str) -> Result<String, String> {
    if raw.is_empty() || raw.len() > MAX_USERNAME {
        return Err("username must be 1-32 characters".into());
    }
    let mut chars = raw.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphanumeric() || first == '_') {
        return Err("username must start with a letter, digit or underscore".into());
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-') {
        return Err("username contains an unsupported character".into());
    }
    Ok(raw.into())
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut tty = None;
    let mut login = PathBuf::from(DEFAULT_LOGIN);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--tty" => {
                tty = Some(PathBuf::from(args.get(i + 1).ok_or("--tty needs a value")?));
                i += 2;
            }
            "--login" => {
                login = PathBuf::from(args.get(i + 1).ok_or("--login needs a value")?);
                i += 2;
            }
            other => return Err(format!("unknown argument: {}", other)),
        }
    }
    Ok(Options { tty, login })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_plain_username_is_accepted() {
        assert_eq!(validate_username("alice").unwrap(), "alice");
        assert_eq!(validate_username("_svc-1.x").unwrap(), "_svc-1.x");
    }

    #[test]
    fn an_injection_shaped_username_is_refused() {
        for bad in ["", "-o", "a b", "a/b", "root;reboot", "üser"] {
            assert!(validate_username(bad).is_err(), "accepted {:?}", bad);
        }
    }

    #[test]
    fn login_always_gets_preserve_environment_and_one_username() {
        assert_eq!(login_args("alice"), ["-p".to_string(), "alice".to_string()]);
    }

    #[test]
    fn the_tty_and_login_are_parsed() {
        let options = parse_args(&args(&["--tty", "/dev/tty1", "--login", "/bin/login"])).unwrap();
        assert_eq!(options.tty, Some(PathBuf::from("/dev/tty1")));
        assert_eq!(options.login, PathBuf::from("/bin/login"));
    }

    #[test]
    fn an_unknown_argument_is_refused() {
        assert!(parse_args(&args(&["--frobnicate"])).is_err());
    }
}
