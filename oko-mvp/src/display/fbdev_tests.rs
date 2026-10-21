//! Linux fbdev format detection, without a device.

use super::fb::PixelFormat;
use super::fbdev::{format_for, FbBitfield, FbVarScreeninfo};

fn var(bpp: u32, red: u32, green: u32, blue: u32) -> FbVarScreeninfo {
    let mut var: FbVarScreeninfo = unsafe { std::mem::zeroed() };
    var.bits_per_pixel = bpp;
    var.red = FbBitfield {
        offset: red,
        length: 0,
        msb_right: 0,
    };
    var.green = FbBitfield {
        offset: green,
        length: 0,
        msb_right: 0,
    };
    var.blue = FbBitfield {
        offset: blue,
        length: 0,
        msb_right: 0,
    };
    var
}

#[test]
fn bgra_offsets_are_recognised() {
    assert_eq!(format_for(&var(32, 16, 8, 0)), Ok(PixelFormat::Bgra8888));
}

#[test]
fn rgba_offsets_are_recognised() {
    assert_eq!(format_for(&var(32, 0, 8, 16)), Ok(PixelFormat::Rgba8888));
}

#[test]
fn rgb565_is_recognised() {
    assert_eq!(format_for(&var(16, 11, 5, 0)), Ok(PixelFormat::Rgb565));
}

#[test]
fn an_unknown_layout_is_refused() {
    assert!(format_for(&var(24, 0, 8, 16)).is_err());
}
