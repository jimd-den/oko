# 0005 — The display server is surfaces and presenters, not a browser

Status: accepted · Date: 2026-10-20

## Context

ỌKỌ is meant to be a display server for esoteric UIs on a barebones Unix: no
X11, no Wayland, no compositor already running, and in the first target, a
console TTY that `getty` would otherwise own. The UI language should still be
HTML, CSS and JavaScript, and apps should be able to present something that is
not a web page at all.

Two facts shaped this decision:

- The pinned Servo build renders offscreen with no display server on this
  machine. With `DISPLAY` and `WAYLAND_DISPLAY` unset, `servoshell -z -x -o
  frame.bmp <url>` completes in about a second through Mesa llvmpipe. The first
  cold run hung; repeat runs are reliable. So a web renderer does not have to
  own a window or a display server.
- A terminal can carry a pixel surface: one cell holds two vertical pixels as
  a truecolor half block. A composed frame therefore has a presenter that works
  over a serial console today, with no device beyond the TTY.

## Decision

**The display server is a surface contract with swappable presenters.**

- An app owns a `Surface`: RGBA8888 pixels, a position, a z-order and
  visibility. It does not know the output device.
- The `Compositor` stacks visible surfaces by z-order, clips them to the output
  and blends them into one `Frame`. It is pure: the same scene produces the
  same frame.
- A `Presenter` puts a `Frame` on a device. The first two are `HeadlessPresenter`
  (tests and tools) and `TerminalPresenter` (truecolor ANSI half blocks). A
  framebuffer, DRM/KMS, wscons, X11/Wayland or browser-canvas presenter
  implements the same trait.
- **Servo is a renderer, not the display server.** A web app is rendered into a
  surface; the display server composites and presents it. A terminal grid or a
  native canvas is another surface producer. The app chooses the producer; the
  presenter chooses the device.
- The core and the terminal presenter never depend on Servo, a display server
  or a GPU. Login and recovery can therefore run on a bare TTY.

## Alternatives

- **A pure TUI.** Rejected: the owner wants HTML/CSS/JS UIs and apps that are
  not terminal widgets. A TUI remains a fallback, not the platform.
- **Embed Servo as the shell.** Rejected: the engine would own the display,
  input and lifecycle, and the platform would become a browser wrapper.
- **Require X11 or Wayland.** Rejected: the target has neither, and a display
  server is the thing ỌKỌ is replacing.
- **Write a full HTML/CSS/JS engine immediately.** Rejected: too large to be a
  first slice. The surface contract lets the renderer stay an adapter while the
  display server is built and tested.

## Consequences

- The terminal presenter is the bare-metal baseline. Its resolution is two
  pixels per cell, which suits large text, dashboards and abstract visuals, not
  a normal web page. Sixel or Kitty graphics can raise it where supported.
- A framebuffer or DRM presenter shows the same frame at full resolution. The
  app does not change.
- Live web interaction still needs Servo's devtools protocol; the one-shot
  render path is static. That is the next hinge, not a prerequisite for the
  display core.
- The prebuilt Servo is Linux aarch64. BSD needs a Servo build or a non-web
  renderer; the display core and terminal/native surfaces remain portable.
- Surface transport, damage rectangles, shared memory and input routing are not
  decided here. They are the next records.

## Revisit if

The terminal presenter cannot show the UIs people actually want, or a browser
engine ships a supported offscreen/DRM backend that makes a full-resolution
web surface cheaper than the current adapter.
