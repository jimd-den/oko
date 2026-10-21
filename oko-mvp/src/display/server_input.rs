//! Input pumping for a surface connection.
//!
//! The display server owns the input device. This module drains that source and
//! writes each event to the focused surface, with one reserved key checked
//! before routing so an app cannot capture the escape.

use super::presenter::Presenter;
use super::protocol::{InputEvent, ServerMessage};
use super::server::reply_to;
use super::session::DisplaySession;
use super::wire::{read_client, write_server};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError};

/// Serve one stream and pump its input source to the focused surface.
///
/// `poll` gives the loop a bounded wait: while the client sends nothing, the
/// server wakes every 16 ms and drains input. Input is written on the same
/// thread as replies, so frames cannot interleave.
/// Which key the server handles before routing. `Ctrl-]` is the default: it is
/// available on every terminal and rare in ordinary typing. An app cannot
/// capture it, because the check happens before focus routing.
#[derive(Debug, Clone, Copy)]
pub struct InputPolicy {
    pub quit: u32,
}

impl Default for InputPolicy {
    fn default() -> Self {
        Self { quit: 0x1d }
    }
}

/// Why a connection ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServeEnd {
    Disconnected,
    Quit,
}

pub fn serve_stream_with_input<P: Presenter>(
    stream: UnixStream,
    session: &mut DisplaySession<P>,
    input: &Receiver<InputEvent>,
    policy: &InputPolicy,
) -> Result<ServeEnd, String> {
    let mut reader = stream.try_clone().map_err(|e| e.to_string())?;
    let mut writer = stream;
    let fd = writer.as_raw_fd();
    loop {
        if !drain_input(&mut writer, session, input, policy)? {
            return Ok(ServeEnd::Quit);
        }
        let mut descriptor = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut descriptor, 1, 16) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(format!("poll: {}", error));
        }
        if ready == 0 {
            continue;
        }
        if descriptor.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) == 0 {
            continue;
        }
        let message = match read_client(&mut reader)? {
            Some(message) => message,
            None => return Ok(ServeEnd::Disconnected),
        };
        let reply = reply_to(session, message);
        write_server(&mut writer, &reply)?;
    }
}

fn drain_input<W: Write, P: Presenter>(
    writer: &mut W,
    session: &DisplaySession<P>,
    input: &Receiver<InputEvent>,
    policy: &InputPolicy,
) -> Result<bool, String> {
    loop {
        match input.try_recv() {
            Ok(InputEvent::Key {
                code,
                pressed: true,
            }) if code == policy.quit => return Ok(false),
            Ok(event) => {
                if let Some(id) = session.focused() {
                    write_server(writer, &ServerMessage::Input { id, event })?;
                }
            }
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return Ok(true),
        }
    }
}

/// Like `serve_unix`, but each connection receives routed input events.
pub fn serve_unix_with_input<P: Presenter>(
    path: &Path,
    session: &mut DisplaySession<P>,
    input: &Receiver<InputEvent>,
    policy: &InputPolicy,
) -> Result<(), String> {
    let listener = UnixListener::bind(path)
        .map_err(|e| format!("bind {}: {}", path.display(), e))?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => match serve_stream_with_input(stream, session, input, policy) {
                Ok(ServeEnd::Quit) => return Ok(()),
                Ok(ServeEnd::Disconnected) => {}
                Err(error) => eprintln!("surface client: {}", error),
            },
            Err(error) => eprintln!("surface accept: {}", error),
        }
    }
    Ok(())
}
