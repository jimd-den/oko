//! BMP frames: the format the pinned Servo build writes for `-o`.
//!
//! Only uncompressed 24-bit and 32-bit BITFIELDS images are accepted. That is
//! what `servoshell -o` produces, and it is enough to turn an offscreen render
//! into a `Frame` without an image crate. Anything else is refused, not
//! guessed at.

use super::surface::Frame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BmpError {
    NotBmp,
    Truncated,
    Unsupported(String),
    BadSize,
}

pub fn decode_bmp(data: &[u8]) -> Result<Frame, BmpError> {
    if data.len() < 54 || &data[0..2] != b"BM" {
        return Err(BmpError::NotBmp);
    }
    let offset = u32_at(data, 10)? as usize;
    let header = u32_at(data, 14)? as usize;
    if header < 40 {
        return Err(BmpError::Unsupported("BITMAPCOREHEADER".into()));
    }
    let width = i32_at(data, 18)?;
    let height = i32_at(data, 22)?;
    let bpp = u16_at(data, 28)?;
    let compression = u32_at(data, 30)?;
    if width <= 0 || height == 0 {
        return Err(BmpError::BadSize);
    }
    let w = width as u32;
    let h = height.unsigned_abs();
    let step = match bpp {
        24 => 3,
        32 => 4,
        other => return Err(BmpError::Unsupported(format!("{} bpp", other))),
    };
    let masks = masks(data, header, bpp, compression)?;
    let stride = (w as usize * bpp as usize).div_ceil(32) * 4;
    let end = offset
        .checked_add(stride.checked_mul(h as usize).ok_or(BmpError::Truncated)?)
        .ok_or(BmpError::Truncated)?;
    if data.len() < end {
        return Err(BmpError::Truncated);
    }
    let top_down = height < 0;
    let mut frame = Frame::new(w, h);
    for y in 0..h {
        let sy = if top_down { y } else { h - 1 - y };
        for x in 0..w {
            let i = offset + sy as usize * stride + x as usize * step;
            let v = if step == 4 {
                u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]])
            } else {
                u32::from_le_bytes([data[i], data[i + 1], data[i + 2], 0])
            };
            let [rm, gm, bm] = masks;
            let rgb = [channel(v, rm), channel(v, gm), channel(v, bm), 255];
            frame.set_pixel(x, y, rgb);
        }
    }
    Ok(frame)
}

fn masks(data: &[u8], header: usize, bpp: u16, compression: u32) -> Result<[u32; 3], BmpError> {
    match compression {
        0 if bpp == 24 => Ok([0x00FF0000, 0x0000FF00, 0x000000FF]),
        0 if bpp == 32 => Ok([0x00FF0000, 0x0000FF00, 0x000000FF]),
        3 if bpp == 32 && header >= 52 => Ok([
            u32_at(data, 54)?,
            u32_at(data, 58)?,
            u32_at(data, 62)?,
        ]),
        other => Err(BmpError::Unsupported(format!(
            "compression {} with {} bpp",
            other, bpp
        ))),
    }
}

fn channel(value: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let max = mask >> shift;
    ((((value & mask) >> shift) * 255) / max) as u8
}

fn u16_at(data: &[u8], offset: usize) -> Result<u16, BmpError> {
    let bytes = data.get(offset..offset + 2).ok_or(BmpError::Truncated)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn u32_at(data: &[u8], offset: usize) -> Result<u32, BmpError> {
    let bytes = data.get(offset..offset + 4).ok_or(BmpError::Truncated)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn i32_at(data: &[u8], offset: usize) -> Result<i32, BmpError> {
    Ok(u32_at(data, offset)? as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(width: i32, height: i32, bpp: u16, compression: u32, pixels: usize) -> Vec<u8> {
        let mut out = vec![0u8; 54];
        out[0..2].copy_from_slice(b"BM");
        out[2..6].copy_from_slice(&((54 + pixels) as u32).to_le_bytes());
        out[10..14].copy_from_slice(&54u32.to_le_bytes());
        out[14..18].copy_from_slice(&40u32.to_le_bytes());
        out[18..22].copy_from_slice(&width.to_le_bytes());
        out[22..26].copy_from_slice(&height.to_le_bytes());
        out[26..28].copy_from_slice(&1u16.to_le_bytes());
        out[28..30].copy_from_slice(&bpp.to_le_bytes());
        out[30..34].copy_from_slice(&compression.to_le_bytes());
        out
    }

    #[test]
    fn a_24_bit_bmp_decodes_top_to_bottom() {
        let mut data = header(2, 1, 24, 0, 8);
        data.extend_from_slice(&[0, 0, 255]); // red in BGR
        data.extend_from_slice(&[255, 0, 0]); // blue in BGR
        data.extend_from_slice(&[0, 0]); // row padding
        let frame = decode_bmp(&data).unwrap();
        assert_eq!(frame.pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(frame.pixel(1, 0), Some([0, 0, 255, 255]));
    }

    #[test]
    fn a_32_bit_bitfields_bmp_decodes_with_masks() {
        let mut data = vec![0u8; 126];
        data[0..2].copy_from_slice(b"BM");
        data[10..14].copy_from_slice(&122u32.to_le_bytes());
        data[14..18].copy_from_slice(&108u32.to_le_bytes());
        data[18..22].copy_from_slice(&1i32.to_le_bytes());
        data[22..26].copy_from_slice(&1i32.to_le_bytes());
        data[28..30].copy_from_slice(&32u16.to_le_bytes());
        data[30..34].copy_from_slice(&3u32.to_le_bytes());
        data[54..58].copy_from_slice(&0x00FF0000u32.to_le_bytes());
        data[58..62].copy_from_slice(&0x0000FF00u32.to_le_bytes());
        data[62..66].copy_from_slice(&0x000000FFu32.to_le_bytes());
        data[122..126].copy_from_slice(&[0x56, 0x34, 0x12, 0xFF]); // B, G, R, A
        let frame = decode_bmp(&data).unwrap();
        assert_eq!(frame.pixel(0, 0), Some([0x12, 0x34, 0x56, 255]));
    }

    #[test]
    fn a_file_that_is_not_a_bmp_is_refused() {
        assert_eq!(decode_bmp(b"not a bitmap"), Err(BmpError::NotBmp));
    }

    #[test]
    fn a_truncated_bmp_is_refused() {
        let data = header(2, 2, 24, 0, 16);
        assert_eq!(decode_bmp(&data), Err(BmpError::Truncated));
    }
}
