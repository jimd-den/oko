# Servo crate probe — API valid, host build blocked

Date: 2026-10-21. Host: aarch64 Linux container, 2.7 GB RAM, ~11 GB free disk.
Servo crate: 0.7.0 from crates.io (same version as the prebuilt servoshell).

## Question

Can ỌKỌ use the published `servo` crate with `SoftwareRenderingContext` to
produce web frames with no display server?

## API findings — verified from the v0.7.0 source

- `SoftwareRenderingContext::new(PhysicalSize)` creates a surfman **software**
  adapter surface. No X11 or Wayland.
- `RenderingContext::read_to_image(DeviceIntRect) -> Option<RgbaImage>`.
- `WebView::paint()` renders into the context.
- `WebView::take_screenshot(None, callback)` waits for load/layout and returns
  `Result<RgbaImage, ScreenshotCaptureError>`.
- `WebView::notify_input_event(InputEvent)` accepts keyboard, mouse, wheel and
  touch events. `KeyboardEvent` wraps `keyboard_types::KeyboardEvent`.
- `servo::InputEvent` and `keyboard_types` are re-exported by the crate.

This is the frame sink the prebuilt devtools does not have.

## Build probe

Source: `/home/violindocgfx/servo-probe` (also copied to `spikes/servo-embed`).
Cargo.toml:

    servo = { version = "0.7.0", default-features = false, features = ["bundled", "js_jit"] }
    dpi = "0.1"
    url = "2.5"
    rustls = { version = "0.23", default-features = false, features = ["aws_lc_rs"] }

The probe creates a `SoftwareRenderingContext(320x200)`, builds a `WebView` on a
`data:` URL, calls `take_screenshot`, spins the event loop, and writes a PPM.

### Result

`cargo check` did not finish. It reached `stylo 0.22.0` and aborted:

    memory allocation of 2304 bytes failed
    error: could not compile `stylo` (lib)  (SIGABRT)

Retry with `CARGO_PROFILE_DEV_DEBUG=0`, incremental off, and a 2.4 GB cap:

    memory allocation of 2654216 bytes failed
    error: could not compile `stylo` (lib)  (SIGABRT)

The failure is the address-space cap, not a Rust type error. `stylo` needs more
than 2.4 GB of virtual address space to type-check. The host has 2.7 GB total,
so there is no safe headroom to raise it further without risking the whole
container.

### Other build facts

- `freetype-sys` builds bundled libpng and needs `zlib.h`; `zlib1g-dev` was
  installed to get past it.
- `mozjs 0.26.4` can link a prebuilt SpiderMonkey archive; it does not have to
  compile SpiderMonkey from source.
- The crate graph and all dependencies downloaded successfully. The blocker is
  compiling `stylo`, not resolving or fetching.

## Conclusion

The API is viable. The host is not. Building the embedder needs a machine with
enough RAM for `stylo` and the Servo link — budget at least 8 GB, measure on
the actual build host. The default ỌKỌ build must not depend on this crate; the
embedder is a separate process and a separate build (ADR 0009).

The next step is to run `spikes/servo-embed` on that larger host. If it writes a
frame with no `DISPLAY`/`WAYLAND_DISPLAY`, the embedder can be wired to
`oko-display` with the existing surface protocol.
