//! One PTY session: the master side, its reader thread, and the child.
//!
//! Kept apart from the adapter so the adapter is about the port contract — open,
//! write, read, resize, report — and this is about the mechanics of one session.
//!
//! Hazard: the reader thread lives until the PTY closes. It is detached on
//! purpose and owns nothing but its buffer, so a session dropped without a close
//! leaks one thread until the shell exits.

use super::grid::Screen;
use crate::ports::terminal::PtyError;
use portable_pty::{Child, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long the screen must be unchanged before a read believes the shell has
/// finished drawing. A shell writes its echo first and the command's output
/// after, so a read that stopped at the first byte would answer with the echo.
const QUIET: Duration = Duration::from_millis(400);

/// The longest a read waits for anything at all. A session that says nothing
/// still answers; it does not hold its caller. The first read of a fresh shell
/// waits the whole cap, because a cold shell may not have drawn its prompt
/// inside the quiet window (that race made a console test flaky).
const SETTLE_CAP: Duration = Duration::from_millis(2000);

const POLL: Duration = Duration::from_millis(10);

pub(super) struct SessionEntry {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output: Arc<Mutex<Vec<u8>>>,
    screen: Screen,
    child: Box<dyn Child + Send + Sync>,
    last_exit: Option<i32>,
}

impl SessionEntry {
    pub(super) fn new(
        master: Box<dyn MasterPty + Send>,
        writer: Box<dyn Write + Send>,
        mut reader: Box<dyn Read + Send>,
        child: Box<dyn Child + Send + Sync>,
        cols: u16,
        rows: u16,
    ) -> Self {
        let output = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&output);
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if let Ok(mut out) = sink.lock() {
                    out.extend_from_slice(&buf[..n]);
                }
            }
        });
        let mut entry = Self {
            master,
            writer,
            output,
            screen: Screen::new(cols, rows),
            child,
            last_exit: None,
        };
        entry.wait_for_first_draw();
        entry
    }

    /// Wait, bounded, for the shell to draw its first prompt.
    ///
    /// A session that accepts input before the shell has started buffers that
    /// input, so the first command's echo and answer can arrive in a different
    /// order than a caller expects. Waiting here makes "session created" mean
    /// "shell is ready to read", not merely "PTY is open".
    fn wait_for_first_draw(&mut self) {
        let deadline = Instant::now() + SETTLE_CAP;
        while Instant::now() < deadline {
            match self.take_output() {
                Ok(bytes) if !bytes.is_empty() => {
                    self.screen.feed(&bytes);
                    return;
                }
                Ok(_) => std::thread::sleep(POLL),
                Err(_) => return,
            }
        }
    }

    /// Everything the shell has written since the last read.
    fn take_output(&self) -> Result<Vec<u8>, PtyError> {
        let mut out = self
            .output
            .lock()
            .map_err(|_| PtyError::ReadFailed("buffer lock poisoned".into()))?;
        Ok(std::mem::take(&mut *out))
    }

    pub(super) fn write(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.writer
            .write_all(bytes)
            .and_then(|_| self.writer.flush())
            .map_err(|e| PtyError::WriteFailed(e.to_string()))
    }

    /// The screen the session is showing, once it has stopped changing.
    ///
    /// Waits for quiet rather than for the first byte, because a shell draws the
    /// echo of what it was given before it draws the answer. The wait is
    /// bounded, so a session with nothing to say still returns.
    pub(super) fn read_screen(&mut self) -> Result<String, PtyError> {
        let deadline = Instant::now() + SETTLE_CAP;
        let mut quiet_since = Instant::now();
        let mut drew_something = false;
        loop {
            let bytes = self.take_output()?;
            if bytes.is_empty() {
                // Quiet only means "finished" once the session has drawn at
                // least once. A cold shell is silent while it starts, and that
                // is not an empty answer.
                if drew_something && quiet_since.elapsed() >= QUIET {
                    break;
                }
            } else {
                self.screen.feed(&bytes);
                drew_something = true;
                quiet_since = Instant::now();
            }
            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(POLL);
        }
        Ok(self.screen.render())
    }

    /// The row the cursor is on: the line the shell will read into next.
    pub(super) fn cursor_line(&self) -> usize {
        self.screen.cursor_line()
    }

    pub(super) fn resize(&mut self, cols: u16, rows: u16) -> Result<(), PtyError> {
        self.screen.resize(cols, rows);
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::ReadFailed(format!("resize: {}", e)))
    }

    /// The session's status, once its shell has ended (TERM-008).
    pub(super) fn exit_status(&mut self) -> Result<Option<i32>, PtyError> {
        if self.last_exit.is_none() {
            if let Ok(Some(status)) = self.child.try_wait() {
                self.last_exit = Some(status.exit_code() as i32);
            }
        }
        Ok(self.last_exit)
    }
}
