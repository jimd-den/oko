//! Surface server: the display contract over a Unix socket.
//!
//! One connection at a time. A connection owns the session's scene; when it
//! disconnects, the surfaces it created stay until it destroys them or a later
//! connection removes them. That is what makes a UI reload cheap, and it is
//! also why a client must name its surfaces explicitly.
//!
//! Input is owned by the display server, not by an app. The server drains its
//! input source and writes each event to the client that currently owns focus.
//! An app never opens the keyboard device.

use super::presenter::Presenter;
use super::protocol::{ClientMessage, InputEvent, ServerMessage};
use super::session::DisplaySession;
use super::wire::{read_client, write_server};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError};

pub fn serve_connection<R: Read, W: Write, P: Presenter>(
    reader: &mut R,
    writer: &mut W,
    session: &mut DisplaySession<P>,
) -> Result<(), String> {
    loop {
        let message = match read_client(reader)? {
            Some(message) => message,
            None => return Ok(()),
        };
        let reply = reply_to(session, message);
        write_server(writer, &reply)?;
    }
}

fn reply_to<P: Presenter>(
    session: &mut DisplaySession<P>,
    message: ClientMessage,
) -> ServerMessage {
    match session.apply(message) {
        Ok(reply @ ServerMessage::Committed { .. }) => match session.present() {
            Ok(()) => reply,
            Err(error) => ServerMessage::Error {
                reason: format!("present: {:?}", error),
            },
        },
        Ok(reply) => reply,
        Err(error) => ServerMessage::Error {
            reason: error.reason(),
        },
    }
}

pub fn serve_stream<P: Presenter>(
    stream: UnixStream,
    session: &mut DisplaySession<P>,
) -> Result<(), String> {
    let mut reader = stream.try_clone().map_err(|e| e.to_string())?;
    let mut writer = stream;
    serve_connection(&mut reader, &mut writer, session)
}

/// Serve one stream and pump its input source to the focused surface.
///
/// `poll` gives the loop a bounded wait: while the client sends nothing, the
/// server wakes every 16 ms and drains input. Input is written on the same
/// thread as replies, so frames cannot interleave.
pub fn serve_stream_with_input<P: Presenter>(
    stream: UnixStream,
    session: &mut DisplaySession<P>,
    input: &Receiver<InputEvent>,
) -> Result<(), String> {
    let mut reader = stream.try_clone().map_err(|e| e.to_string())?;
    let mut writer = stream;
    let fd = writer.as_raw_fd();
    loop {
        drain_input(&mut writer, session, input)?;
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
            None => return Ok(()),
        };
        let reply = reply_to(session, message);
        write_server(&mut writer, &reply)?;
    }
}

fn drain_input<W: Write, P: Presenter>(
    writer: &mut W,
    session: &DisplaySession<P>,
    input: &Receiver<InputEvent>,
) -> Result<(), String> {
    let Some(id) = session.focused() else {
        while input.try_recv().is_ok() {}
        return Ok(());
    };
    loop {
        match input.try_recv() {
            Ok(event) => write_server(writer, &ServerMessage::Input { id, event })?,
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return Ok(()),
        }
    }
}

/// Bind `path`, serve exactly one connection, then return.
///
/// The caller owns the path and must remove it if it wants to bind again.
pub fn serve_unix_once<P: Presenter>(
    path: &Path,
    session: &mut DisplaySession<P>,
) -> Result<(), String> {
    let listener = UnixListener::bind(path)
        .map_err(|e| format!("bind {}: {}", path.display(), e))?;
    let (stream, _) = listener
        .accept()
        .map_err(|e| format!("accept: {}", e))?;
    serve_stream(stream, session)
}

/// Bind `path` and serve connections until the process is stopped.
pub fn serve_unix<P: Presenter>(path: &Path, session: &mut DisplaySession<P>) -> Result<(), String> {
    let listener = UnixListener::bind(path)
        .map_err(|e| format!("bind {}: {}", path.display(), e))?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = serve_stream(stream, session) {
                    eprintln!("surface client: {}", error);
                }
            }
            Err(error) => eprintln!("surface accept: {}", error),
        }
    }
    Ok(())
}

/// Like `serve_unix`, but each connection receives routed input events.
pub fn serve_unix_with_input<P: Presenter>(
    path: &Path,
    session: &mut DisplaySession<P>,
    input: &Receiver<InputEvent>,
) -> Result<(), String> {
    let listener = UnixListener::bind(path)
        .map_err(|e| format!("bind {}: {}", path.display(), e))?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = serve_stream_with_input(stream, session, input) {
                    eprintln!("surface client: {}", error);
                }
            }
            Err(error) => eprintln!("surface accept: {}", error),
        }
    }
    Ok(())
}
