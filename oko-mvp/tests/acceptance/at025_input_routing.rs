//! Acceptance test AT-025: input goes only to the focused surface.
//!
//! A socket pair stands in for an app process. The server has a real input
//! channel; the client creates one surface, which takes focus, then the test
//! sends a key and reads it back as an input frame on that surface.

use oko_core::display::presenter::HeadlessPresenter;
use oko_core::display::protocol::{ClientMessage, InputEvent, ServerMessage};
use oko_core::display::server::serve_stream_with_input;
use oko_core::display::session::DisplaySession;
use oko_core::display::surface::SurfaceId;
use oko_core::display::wire::{read_server, write_client};
use std::os::unix::net::UnixStream;
use std::sync::mpsc::channel;

#[test]
fn a_key_reaches_the_focused_surface() {
    let (server_side, mut client) = UnixStream::pair().unwrap();
    let (input_tx, input_rx) = channel();
    let server = std::thread::spawn(move || {
        let mut session = DisplaySession::new(1, 1, HeadlessPresenter::new(1, 1));
        serve_stream_with_input(server_side, &mut session, &input_rx)
    });

    let mut reader = client.try_clone().unwrap();
    write_client(
        &mut client,
        &ClientMessage::Create {
            id: SurfaceId(1),
            width: 1,
            height: 1,
        },
    )
    .unwrap();
    assert_eq!(
        read_server(&mut reader).unwrap(),
        Some(ServerMessage::Created { id: SurfaceId(1) })
    );

    input_tx
        .send(InputEvent::Key {
            code: 0x41,
            pressed: true,
        })
        .unwrap();

    let message = match read_server(&mut reader).unwrap() {
        Some(ServerMessage::Input { id, event }) => (id, event),
        other => panic!("expected input, got {:?}", other),
    };
    assert_eq!(
        message,
        (
            SurfaceId(1),
            InputEvent::Key {
                code: 0x41,
                pressed: true
            }
        )
    );

    drop(client);
    drop(reader);
    server.join().unwrap().unwrap();
}
