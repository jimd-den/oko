//! Display core tests: pure surfaces, composition, and the two presenters.
//!
//! These tests never touch a device, a clock, or the network. A presenter is
//! driven through its public trait, so the terminal and headless paths are
//! tested exactly as the host drives them.

use super::*;

fn red() -> [u8; 4] {
    [255, 0, 0, 255]
}

fn blue() -> [u8; 4] {
    [0, 0, 255, 255]
}

fn black() -> [u8; 4] {
    [0, 0, 0, 255]
}

#[test]
fn a_new_surface_is_transparent() {
    let surface = Surface::new(SurfaceId(1), 2, 2);
    assert_eq!(surface.pixel(0, 0), Some([0, 0, 0, 0]));
}

#[test]
fn a_pixel_written_to_a_surface_is_read_back() {
    let mut surface = Surface::new(SurfaceId(1), 2, 2);
    assert!(surface.set_pixel(1, 1, red()));
    assert_eq!(surface.pixel(1, 1), Some(red()));
}

#[test]
fn a_pixel_outside_the_surface_is_refused() {
    let mut surface = Surface::new(SurfaceId(1), 2, 2);
    assert!(!surface.set_pixel(2, 0, red()));
    assert_eq!(surface.pixel(2, 0), None);
}

#[test]
fn clear_fills_every_pixel() {
    let mut surface = Surface::new(SurfaceId(1), 2, 2);
    surface.clear(red());
    assert_eq!(surface.pixel(0, 0), Some(red()));
    assert_eq!(surface.pixel(1, 1), Some(red()));
}

#[test]
fn an_empty_scene_is_the_background() {
    let compositor = Compositor::new(2, 2).with_background([10, 20, 30, 255]);
    let frame = compositor.composite();
    assert_eq!(frame.pixel(0, 0), Some([10, 20, 30, 255]));
    assert_eq!(frame.pixel(1, 1), Some([10, 20, 30, 255]));
}

#[test]
fn a_surface_appears_at_its_position() {
    let mut compositor = Compositor::new(3, 3);
    let mut surface = Surface::new(SurfaceId(1), 1, 1);
    surface.set_pixel(0, 0, red());
    surface.set_position(1, 2);
    compositor.add(surface);
    let frame = compositor.composite();
    assert_eq!(frame.pixel(1, 2), Some(red()));
    assert_eq!(frame.pixel(0, 0), Some(black()));
}

#[test]
fn a_higher_surface_covers_a_lower_one() {
    let mut lower = Surface::new(SurfaceId(1), 1, 1);
    lower.set_pixel(0, 0, red());
    lower.set_z(0);
    let mut upper = Surface::new(SurfaceId(2), 1, 1);
    upper.set_pixel(0, 0, blue());
    upper.set_z(1);
    let mut compositor = Compositor::new(1, 1);
    compositor.add(upper);
    compositor.add(lower);
    assert_eq!(compositor.composite().pixel(0, 0), Some(blue()));
}

#[test]
fn a_surface_is_clipped_to_the_output() {
    let mut surface = Surface::new(SurfaceId(1), 2, 2);
    surface.set_pixel(1, 1, red());
    surface.set_position(-1, -1);
    let mut compositor = Compositor::new(1, 1);
    compositor.add(surface);
    assert_eq!(compositor.composite().pixel(0, 0), Some(red()));
}

#[test]
fn a_transparent_pixel_lets_the_background_through() {
    let mut surface = Surface::new(SurfaceId(1), 1, 1);
    surface.clear([255, 0, 0, 0]);
    let mut compositor = Compositor::new(1, 1).with_background([10, 20, 30, 255]);
    compositor.add(surface);
    assert_eq!(compositor.composite().pixel(0, 0), Some([10, 20, 30, 255]));
}

#[test]
fn headless_keeps_the_last_frame() {
    let mut presenter = HeadlessPresenter::new(1, 1);
    let mut frame = Frame::new(1, 1);
    frame.set_pixel(0, 0, red());
    presenter.present(&frame).unwrap();
    assert_eq!(presenter.last_frame().unwrap().pixel(0, 0), Some(red()));
}

#[test]
fn headless_refuses_a_frame_of_the_wrong_size() {
    let mut presenter = HeadlessPresenter::new(2, 2);
    let frame = Frame::new(1, 1);
    assert_eq!(
        presenter.present(&frame),
        Err(PresenterError::WrongSize {
            expected: (2, 2),
            got: (1, 1)
        })
    );
}

#[test]
fn the_terminal_presenter_reports_cells() {
    let presenter = TerminalPresenter::new(Vec::new(), 80, 24);
    assert_eq!(presenter.output_size(), OutputSize::Cells(80, 24));
}

#[test]
fn a_top_and_bottom_pixel_become_one_half_block() {
    let mut frame = Frame::new(1, 2);
    frame.set_pixel(0, 0, red());
    frame.set_pixel(0, 1, blue());
    let mut presenter = TerminalPresenter::new(Vec::new(), 1, 1);
    presenter.present(&frame).unwrap();
    let out = String::from_utf8(presenter.into_inner()).unwrap();
    assert!(out.contains("38;2;255;0;0"), "top color missing: {:?}", out);
    assert!(out.contains("48;2;0;0;255"), "bottom color missing: {:?}", out);
    assert!(out.contains('\u{2580}'), "half block missing: {:?}", out);
}

#[test]
fn one_line_per_terminal_row() {
    let frame = Frame::new(2, 4);
    let mut presenter = TerminalPresenter::new(Vec::new(), 1, 2);
    presenter.present(&frame).unwrap();
    let out = String::from_utf8(presenter.into_inner()).unwrap();
    assert_eq!(out.matches('\n').count(), 2);
}

#[test]
fn an_empty_frame_is_refused() {
    let mut presenter = TerminalPresenter::new(Vec::new(), 1, 1);
    assert_eq!(
        presenter.present(&Frame::new(0, 0)),
        Err(PresenterError::EmptyFrame)
    );
}
