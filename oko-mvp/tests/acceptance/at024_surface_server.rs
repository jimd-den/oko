//! Acceptance test AT-024: an app owns a surface over a Unix socket.
//!
//! The client creates, commits and destroys a surface through the wire format.
//! A recording presenter stands in for the screen, so the test proves the
//! compositor actually received the committed pixels.

use oko_core::display::presenter::{OutputSize, Presenter, PresenterError};
use oko_core::display::protocol::{ClientMessage, ServerMessage};
use oko_core::display::server::serve_unix_once;
use oko_core::display::session::DisplaySession;
use oko_core::display::surface::{Frame, SurfaceId};
use oko_core::display::wire::{read_server, write_client};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct RecordingPresenter {
    size: (u32, u32),
    frames: Arc<Mutex<Vec<Frame>>>,
}

impl Presenter for RecordingPresenter {
    fn output_size(&self) -> OutputSize {
        OutputSize::Pixels(self.size.0, self.size.1)
    }

    fn present(&mut self, frame: &Frame) -> Result<(), PresenterError> {
        self.frames.lock().unwrap().push(frame.clone());
        Ok(())
    }
}

fn socket_path() -> PathBuf {
    std::env::temp_dir().join(format!("oko-surface-{}.sock", std::process::id()))
}

fn connect(path: &PathBuf) -> UnixStream {
    for _ in 0..200 {
        if let Ok(stream) = UnixStream::connect(path) {
            return stream;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("server did not bind {}", path.display());
}

#[test]
fn an_app_commits_a_surface_and_the_server_presents_it() {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    let frames = Arc::new(Mutex::new(Vec::new()));
    let presenter = RecordingPresenter {
        size: (2, 2),
        frames: Arc::clone(&frames),
    };
    let server_path = path.clone();
    let server = std::thread::spawn(move || {
        let mut session = DisplaySession::new(2, 2, presenter);
        serve_unix_once(&server_path, &mut session)
    });

    let mut client = connect(&path);
    let mut reader = client.try_clone().unwrap();

    write_client(
        &mut client,
        &ClientMessage::Create {
            id: SurfaceId(1),
            width: 2,
            height: 2,
        },
    )
    .unwrap();
    assert_eq!(
        read_server(&mut reader).unwrap(),
        Some(ServerMessage::Created { id: SurfaceId(1) })
    );

    let mut frame = Frame::new(2, 2);
    frame.set_pixel(0, 0, [255, 0, 0, 255]);
    write_client(
        &mut client,
        &ClientMessage::Commit {
            id: SurfaceId(1),
            frame,
        },
    )
    .unwrap();
    assert_eq!(
        read_server(&mut reader).unwrap(),
        Some(ServerMessage::Committed { id: SurfaceId(1) })
    );

    write_client(
        &mut client,
        &ClientMessage::Destroy {
            id: SurfaceId(1),
        },
    )
    .unwrap();
    assert_eq!(
        read_server(&mut reader).unwrap(),
        Some(ServerMessage::Destroyed { id: SurfaceId(1) })
    );

    drop(client);
    drop(reader);
    server.join().unwrap().unwrap();
    let _ = std::fs::remove_file(&path);

    let frames = frames.lock().unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].pixel(0, 0), Some([255, 0, 0, 255]));
}
