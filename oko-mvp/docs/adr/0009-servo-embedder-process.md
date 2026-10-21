# 0009 — The Servo embedder is a separate process

Status: proposed · Date: 2026-10-21

## Context

The prebuilt `servoshell` devtools protocol can navigate and evaluate script,
but it exposes no frame sink (docs/reports/servo-devtools-spike.md). The
published `servo` crate at 0.7.0 has what is missing:

- `SoftwareRenderingContext`, a surfman software context that needs no display
  server;
- `WebView::paint()` and `RenderingContext::read_to_image()`;
- `WebView::take_screenshot()`, which waits for load/layout;
- `WebView::notify_input_event()` for keyboard and pointer input.

The catch is size. Linking Servo builds a large Rust graph and a prebuilt
SpiderMonkey archive. It is not something the default `oko-host` build should
carry, and a web-content crash must not take the terminal core down.

## Decision

**A separate `oko-servo-embed` process links Servo and speaks the existing
surface protocol to `oko-display`.**

- `oko-host`, `oko-session`, `oko-display` and the display core do not depend on
  the `servo` crate. The default build stays lean and offline.
- `oko-servo-embed` depends on `servo = "=0.7.0"` with
  `default-features = false, features = ["bundled", "js_jit"]`. It owns one
  `SoftwareRenderingContext` and one `WebView`.
- Frames: `notify_new_frame_ready` -> `WebView::paint()` ->
  `RenderingContext::read_to_image()` (or `take_screenshot`) -> `Frame` ->
  `ClientMessage::Commit` on the surface socket.
- Input: `ServerMessage::Input` -> Servo `InputEvent::Keyboard` /
  `MouseMove` / `MouseButton` -> `WebView::notify_input_event()`.
- The process is zone C: web content gets no host authority. It has no
  filesystem or process capability beyond what the web engine itself needs.
- It is built separately, pinned, and recorded in `toolchain/versions.lock`.
  The build host needs the Servo build dependencies and enough RAM/disk; the
  current container needed `zlib1g-dev` even to compile `freetype-sys`.
- The prebuilt `servoshell` + one-shot `oko-preview` remains the fallback and
  the quick visual test.

## Alternatives

- **Link Servo into `oko-host`.** Rejected: a huge default build, a slow start,
  and a browser crash would share the process with the terminal core.
- **Keep using devtools.** Rejected for live frames: it has no screenshot or
  capture method.
- **Write a custom HTML/CSS/JS renderer.** Rejected: that is a browser project,
  not a display-server slice.
- **Require a system browser.** Rejected: the target has no display server and
  may have no browser.

## Consequences

- Two build profiles: lean core, optional Servo embedder.
- The embedder is Linux-first; BSD depends on Servo and SpiderMonkey support.
- The display server and input contract do not change; the embedder is just
  another surface client.
- The frame path is CPU-rendered and full-frame. Frame rate must be measured;
  shared memory and damage rectangles are still the performance follow-ups.
- A failed embedder leaves the terminal and recovery path usable.

## Revisit if

Servo publishes a smaller embedder library or a prebuilt shared library, or the
software rendering path cannot sustain the UIs people actually use.
