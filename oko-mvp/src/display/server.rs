//! Surface server: the display contract over a Unix socket.
//!
//! One connection at a time. A connection owns the session's scene; when it
//! disconnects, the surfaces it created stay until it destroys them or a later
//! connection removes them. That is what makes a UI reload cheap, and it is
//! also why a client must name its surfaces explicitly.
//!
//! Input is owned by the display server, not by an app; the input-pumping
//! variant lives in `server_input`. An app never opens the keyboard device.

use super::presenter::Presenter;
use super::protocol::{ClientMessage, ServerMessage};
use super::session::DisplaySession;
use super::wire::{read_client, write_server};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

pub use super::server_input::{
    serve_stream_with_input, serve_unix_with_input, InputPolicy, ServeEnd,
};

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

pub(super) fn reply_to<P: Presenter>(
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
