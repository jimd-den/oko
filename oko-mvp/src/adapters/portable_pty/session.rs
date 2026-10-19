//! One PTY session: the master side, its reader thread, and the child.
//!
//! Kept apart from the adapter so the adapter is about the port contract — open,
//! write, read, resize, report — and this is about the mechanics of one session.
//!
//! Hazard: the reader thread lives until the PTY closes. It is detached on
//! purpose and owns nothing but its buffer, so a session dropped without a close
//! leaks one thread until the shell exits.

use crate::ports::terminal::PtyError;
use portable_pty::{Child, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

pub(super) struct SessionEntry {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output: Arc<Mutex<Vec<u8>>>,
    child: Box<dyn Child + Send + Sync>,
    last_exit: Option<i32>,
}

impl SessionEntry {
    pub(super) fn new(
        master: Box<dyn MasterPty + Send>,
        writer: Box<dyn Write + Send>,
        mut reader: Box<dyn Read + Send>,
        child: Box<dyn Child + Send + Sync>,
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
        Self {
            master,
            writer,
            output,
            child,
            last_exit: None,
        }
    }

    pub(super) fn write(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.writer
            .write_all(bytes)
            .and_then(|_| self.writer.flush())
            .map_err(|e| PtyError::WriteFailed(e.to_string()))
    }

    /// Everything the shell has written since the last read.
    ///
    /// Waits briefly for something to arrive, because a caller that has just
    /// written to a shell and read once gets an empty screen: the shell has not
    /// answered yet. The wait is bounded, so a silent session still returns.
    pub(super) fn drain(&mut self) -> Result<String, PtyError> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(400);
        loop {
            let bytes = {
                let mut out = self
                    .output
                    .lock()
                    .map_err(|_| PtyError::ReadFailed("buffer lock poisoned".into()))?;
                std::mem::take(&mut *out)
            };
            if !bytes.is_empty() || std::time::Instant::now() >= deadline {
                return Ok(String::from_utf8_lossy(&bytes).to_string());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    pub(super) fn resize(&self, cols: u16, rows: u16) -> Result<(), PtyError> {
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
