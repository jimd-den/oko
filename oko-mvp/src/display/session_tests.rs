//! The in-process display session, driven exactly as a transport will drive it.

use super::*;

fn session() -> DisplaySession<HeadlessPresenter> {
    DisplaySession::new(1, 1, HeadlessPresenter::new(1, 1))
}

fn create(id: u64, width: u32, height: u32) -> ClientMessage {
    ClientMessage::Create {
        id: SurfaceId(id),
        width,
        height,
    }
}

fn red_frame() -> Frame {
    let mut frame = Frame::new(1, 1);
    frame.set_pixel(0, 0, [255, 0, 0, 255]);
    frame
}

#[test]
fn a_created_surface_becomes_focused() {
    let mut session = session();
    let reply = session.apply(create(1, 1, 1)).unwrap();
    assert_eq!(reply, ServerMessage::Created { id: SurfaceId(1) });
    assert_eq!(session.focused(), Some(SurfaceId(1)));
}

#[test]
fn a_committed_frame_is_presented() {
    let mut session = session();
    session.apply(create(1, 1, 1)).unwrap();
    session
        .apply(ClientMessage::Commit {
            id: SurfaceId(1),
            frame: red_frame(),
        })
        .unwrap();
    session.present().unwrap();
    assert_eq!(
        session.presenter().last_frame().unwrap().pixel(0, 0),
        Some([255, 0, 0, 255])
    );
}

#[test]
fn a_commit_of_the_wrong_size_is_refused_and_changes_nothing() {
    let mut session = session();
    session.apply(create(1, 1, 1)).unwrap();
    let result = session.apply(ClientMessage::Commit {
        id: SurfaceId(1),
        frame: Frame::new(2, 1),
    });
    assert_eq!(
        result,
        Err(DisplayError::SizeMismatch {
            expected: (1, 1),
            got: (2, 1)
        })
    );
    session.present().unwrap();
    assert_eq!(
        session.presenter().last_frame().unwrap().pixel(0, 0),
        Some([0, 0, 0, 255])
    );
}

#[test]
fn a_duplicate_surface_is_refused() {
    let mut session = session();
    session.apply(create(1, 1, 1)).unwrap();
    assert_eq!(
        session.apply(create(1, 1, 1)),
        Err(DisplayError::SurfaceExists(SurfaceId(1)))
    );
}

#[test]
fn a_zero_sized_surface_is_refused() {
    let mut session = session();
    assert!(matches!(
        session.apply(create(1, 0, 1)),
        Err(DisplayError::TooLarge { .. })
    ));
}

#[test]
fn destroy_removes_the_surface_and_moves_focus() {
    let mut session = session();
    session.apply(create(1, 1, 1)).unwrap();
    session.apply(create(2, 1, 1)).unwrap();
    session.focus(SurfaceId(2));
    assert_eq!(session.focused(), Some(SurfaceId(2)));
    assert_eq!(
        session.apply(ClientMessage::Destroy { id: SurfaceId(2) }),
        Ok(ServerMessage::Destroyed { id: SurfaceId(2) })
    );
    assert_eq!(session.focused(), Some(SurfaceId(1)));
}

#[test]
fn input_goes_to_the_focused_surface() {
    let mut session = session();
    session.apply(create(1, 1, 1)).unwrap();
    let event = InputEvent::Key {
        code: 30,
        pressed: true,
    };
    assert_eq!(
        session.dispatch_input(event.clone()),
        Some((SurfaceId(1), event))
    );
}

#[test]
fn no_focus_means_no_input() {
    let mut session = session();
    let event = InputEvent::Pointer {
        x: 0,
        y: 0,
        buttons: 0,
    };
    assert_eq!(session.dispatch_input(event), None);
}
