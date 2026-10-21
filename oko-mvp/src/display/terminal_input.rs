//! The TTY input source: raw mode, a reader thread, and parsed key events.
//!
//! Raw mode is process-wide for the terminal, so the guard owns the saved
//! termios and restores it on drop. The thread is detached; it ends when stdin
//! closes or the receiver is dropped.
//!
//! This is deliberately not evdev/wscons. Those are the next input adapters;
//! they push the same `InputEvent`s into the same channel.

use super::input::KeyParser;
use super::protocol::InputEvent;
use std::io::Read;
use std::sync::mpsc::{channel, Receiver};

pub struct TerminalInput {
    rx: Receiver<InputEvent>,
    _raw: RawTerminal,
}

impl TerminalInput {
    /// Put stdin in raw mode and start reading keys.
    ///
    /// Returns an error when stdin is not a terminal; a caller that has no TTY
    /// should run the display server without input rather than fail.
    pub fn spawn() -> Result<Self, String> {
        let raw = RawTerminal::enable()?;
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let mut parser = KeyParser::default();
            let mut stdin = std::io::stdin();
            let mut byte = [0u8; 1];
            loop {
                match stdin.read(&mut byte) {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {
                        for event in parser.feed(byte[0]) {
                            if tx.send(event).is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        });
        Ok(Self { rx, _raw: raw })
    }

    pub fn receiver(&self) -> &Receiver<InputEvent> {
        &self.rx
    }
}

struct RawTerminal {
    saved: libc::termios,
}

impl RawTerminal {
    fn enable() -> Result<Self, String> {
        let fd = libc::STDIN_FILENO;
        let mut saved: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(fd, &mut saved) } != 0 {
            return Err(format!(
                "stdin is not a terminal: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut raw = saved;
        unsafe { libc::cfmakeraw(&mut raw) };
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
            return Err(format!(
                "raw mode failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Self { saved })
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.saved);
        }
    }
}
