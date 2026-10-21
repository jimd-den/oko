//! Surface server: the display contract over a Unix socket.
//!
//! One connection at a time. A connection owns the session's scene; when it
//! disconnects, the surfaces it created stay until it destroys them or a later
//! connection removes them. That is what makes a UI reload cheap, and it is
//! also why a client must name its surfaces explicitly.
//!
//! Input is not pumped here yet. `DisplaySession::dispatch_input` is the routing
//! point; an evdev/wscons/terminal adapter will feed it and write the event to
//! the focused connection. The wire vocabulary already carries input frames.

use super::presenter::Presenter;
use super::protocol::ServerMessage;
use super::session::DisplaySession;
use super::wire::{read_client, write_server};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

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
        let reply = match session.apply(message) {
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
        };
        write_server(writer, &reply)?;
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
