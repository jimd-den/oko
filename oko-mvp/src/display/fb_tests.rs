//! Framebuffer presenter tests: conversion, stride and size checks.

use super::fb::*;
use super::presenter::{Presenter, PresenterError};
use super::surface::Frame;
use super::*;

fn info(format: PixelFormat) -> FramebufferInfo {
    FramebufferInfo {
        width: 1,
        height: 1,
        stride: format.bytes_per_pixel(),
        format,
    }
}

fn red_frame() -> Frame {
    let mut frame = Frame::new(1, 1);
    frame.set_pixel(0, 0, [255, 0, 0, 255]);
    frame
}

#[test]
fn bgra_swaps_red_and_blue() {
    let mut presenter = FramebufferPresenter::new(MemoryFramebuffer::new(info(PixelFormat::Bgra8888)));
    presenter.present(&red_frame()).unwrap();
    assert_eq!(presenter.device().bytes(), &[0, 0, 255, 255]);
}

#[test]
fn rgb565_packs_a_red_pixel() {
    let mut presenter = FramebufferPresenter::new(MemoryFramebuffer::new(info(PixelFormat::Rgb565)));
    presenter.present(&red_frame()).unwrap();
    assert_eq!(presenter.device().bytes(), &[0x00, 0xF8]);
}

#[test]
fn rows_use_the_device_stride() {
    let info = FramebufferInfo {
        width: 1,
        height: 2,
        stride: 8,
        format: PixelFormat::Rgba8888,
    };
    let mut frame = Frame::new(1, 2);
    frame.set_pixel(0, 0, [255, 0, 0, 255]);
    frame.set_pixel(0, 1, [0, 0, 255, 255]);
    let mut presenter = FramebufferPresenter::new(MemoryFramebuffer::new(info));
    presenter.present(&frame).unwrap();
    assert_eq!(&presenter.device().bytes()[0..4], &[255, 0, 0, 255]);
    assert_eq!(&presenter.device().bytes()[4..8], &[0, 0, 0, 0]);
    assert_eq!(&presenter.device().bytes()[8..12], &[0, 0, 255, 255]);
}

#[test]
fn a_frame_of_the_wrong_size_is_refused() {
    let mut presenter = FramebufferPresenter::new(MemoryFramebuffer::new(info(PixelFormat::Rgba8888)));
    assert_eq!(
        presenter.present(&Frame::new(2, 1)),
        Err(PresenterError::WrongSize {
            expected: (1, 1),
            got: (2, 1)
        })
    );
}

#[test]
fn the_presenter_reports_pixel_size() {
    let presenter = FramebufferPresenter::new(MemoryFramebuffer::new(info(PixelFormat::Rgba8888)));
    assert_eq!(presenter.output_size(), OutputSize::Pixels(1, 1));
}
