//! Linux fbdev adapter: a `FramebufferDevice` over `/dev/fb0`.
//!
//! This is the no-display-server path on a machine whose kernel exposes a
//! framebuffer. It uses the kernel's `fb_var_screeninfo`/`fb_fix_screeninfo`
//! ABI and `mmap`, nothing else. FreeBSD `vt(4)` and OpenBSD wscons have
//! different ioctls and get their own adapters; the presenter above does not
//! change.
//!
//! Hazard: the FFI layout follows Linux `fb.h`. It is compiled only on Linux
//! and has not been exercised on hardware without `/dev/fb0`; the conversion
//! and stride logic are tested through `MemoryFramebuffer`.

use super::fb::{FramebufferDevice, FramebufferInfo, PixelFormat};
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;

const FBIOGET_VSCREENINFO: libc::c_ulong = 0x4600;
const FBIOGET_FSCREENINFO: libc::c_ulong = 0x4602;

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct FbBitfield {
    pub(super) offset: u32,
    pub(super) length: u32,
    pub(super) msb_right: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct FbVarScreeninfo {
    pub(super) xres: u32,
    pub(super) yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,
    pub(super) bits_per_pixel: u32,
    grayscale: u32,
    pub(super) red: FbBitfield,
    pub(super) green: FbBitfield,
    pub(super) blue: FbBitfield,
    transp: FbBitfield,
    nonstd: u32,
    activate: u32,
    height: u32,
    width: u32,
    accel_flags: u32,
    pixclock: u32,
    left_margin: u32,
    right_margin: u32,
    upper_margin: u32,
    lower_margin: u32,
    hsync_len: u32,
    vsync_len: u32,
    sync: u32,
    vmode: u32,
    rotate: u32,
    colorspace: u32,
    reserved: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FbFixScreeninfo {
    id: [u8; 16],
    smem_start: libc::c_ulong,
    smem_len: u32,
    type_: u32,
    type_aux: u32,
    visual: u32,
    xpanstep: u16,
    ypanstep: u16,
    ywrapstep: u16,
    line_length: u32,
    mmio_start: libc::c_ulong,
    mmio_len: u32,
    accel: u32,
    capabilities: u16,
    reserved: [u16; 2],
}

pub struct LinuxFbdev {
    _file: File,
    map: *mut u8,
    len: usize,
    info: FramebufferInfo,
}

impl LinuxFbdev {
    /// Open `path` (normally `/dev/fb0`) and map it.
    pub fn open(path: &Path) -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| format!("open {}: {}", path.display(), e))?;
        let fd = file.as_raw_fd();
        let mut var: FbVarScreeninfo = unsafe { std::mem::zeroed() };
        let mut fix: FbFixScreeninfo = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(fd, FBIOGET_VSCREENINFO, &mut var) } < 0 {
            return Err(format!("FBIOGET_VSCREENINFO: {}", std::io::Error::last_os_error()));
        }
        if unsafe { libc::ioctl(fd, FBIOGET_FSCREENINFO, &mut fix) } < 0 {
            return Err(format!("FBIOGET_FSCREENINFO: {}", std::io::Error::last_os_error()));
        }
        let format = format_for(&var)?;
        let len = fix.smem_len as usize;
        if len == 0 {
            return Err("framebuffer reports zero length".into());
        }
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if map == libc::MAP_FAILED {
            return Err(format!("mmap: {}", std::io::Error::last_os_error()));
        }
        Ok(Self {
            _file: file,
            map: map as *mut u8,
            len,
            info: FramebufferInfo {
                width: var.xres,
                height: var.yres,
                stride: fix.line_length as usize,
                format,
            },
        })
    }
}

pub(super) fn format_for(var: &FbVarScreeninfo) -> Result<PixelFormat, String> {
    match (var.bits_per_pixel, var.red.offset, var.green.offset, var.blue.offset) {
        (32, 16, 8, 0) => Ok(PixelFormat::Bgra8888),
        (32, 0, 8, 16) => Ok(PixelFormat::Rgba8888),
        (16, 11, 5, 0) => Ok(PixelFormat::Rgb565),
        (bpp, r, g, b) => Err(format!(
            "unsupported framebuffer: {} bpp, rgb offsets {}, {}, {}",
            bpp, r, g, b
        )),
    }
}

impl FramebufferDevice for LinuxFbdev {
    fn info(&self) -> FramebufferInfo {
        self.info
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > self.len {
            return Err(format!("frame is {} bytes, framebuffer is {}", bytes.len(), self.len));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.map, bytes.len());
        }
        Ok(())
    }
}

impl Drop for LinuxFbdev {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.map as *mut libc::c_void, self.len);
        }
    }
}
