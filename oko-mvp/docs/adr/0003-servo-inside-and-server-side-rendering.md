# 0003 — ỌKỌ renders the web itself, and the shell is a view onto it

Status: accepted · Date: 2026-10-19

## Context

ỌKỌ is meant to be a place, not an application:

- a place where the **terminal can make a UI** — type something, get an
  interface, not a log;
- a place where **HTML applications work directly with the computer**, with the
  host API and the capability policy behind them;
- a place where **games can run against server-side Vulkan**, with the machine
  doing the rendering;
- a place that is **minimal, hot-swappable and extremely flexible on computers
  that have nothing**;
- and the interface should be a **real Star Trek computer** — the thing someone
  in that universe would actually use, built with HTML, CSS and JavaScript, not
  a terminal dressed in LCARS colours.

The last two pull against the current implementation in one specific way, and
that is what this record decides.

## The problem with the current shape

The shell is a page served over HTTP and displayed by *some* browser. The Web
pane is an iframe, so the browsing happens in **that** browser: its engine, its
cookies, its framing rules — which is why github, google and youtube refuse to
load in it. The host's Servo is a separate window on the same machine, not part
of ỌKỌ at all.

So today, "ỌKỌ" is a shell that borrows someone else's engine. That is not a
place; it is a page.

## Decision

**ỌKỌ owns web content rendering, whoever is displaying the shell.**

- A page is rendered by **the host's Servo**, headless, never by the browser
  showing the shell. If you open ỌKỌ in Chrome, the content inside is still
  Servo's.
- The host streams **frames** to the shell, and the shell is a *view*: it draws
  pixels and sends input. The same view works in a browser, in a Servo window,
  on a DRM framebuffer (#1), or over a network — the transport changes, the
  view does not.
- **Server-side rendering is the default**, because it is the same decision that
  makes "games on server Vulkan" work: the machine that has the GPU renders,
  the surface that has the screen displays. One pipeline, not two.
- **The UI is a set of files.** `ui/**` is served live; there is no build step
  and no compile. Editing a file and reloading *is* the hot-swap, and restarting
  the host is the blunt version of it. This is what makes UI development easy
  and the whole thing minimal on a machine that has nothing.
- **The terminal is a pane among panes, and a UI factory.** A command can create
  a pane, open a file, or serve an application; a terminal that can make a UI is
  the point, not a feature.

## Consequences

- The Web pane stops being an iframe and becomes a **surface**: a rectangle the
  host paints. That is the same surface the DRM presenter (#1) needs, so the two
  efforts are one effort.
- Servo must be **driven, not restarted**: navigating a live session, resizing
  it, and reading its frames needs its DevTools protocol. That is the piece that
  turns the process embedder into a real one.
- Frame streaming has a cost that must be measured, not assumed: a naive
  full-frame-per-change stream is fine on loopback and wrong over a network.
- The Star Trek interface is then an **application** on this platform: panels,
  readouts, graphics, its own state, built with the same tools as everything
  else. It should not be the shell with different colours.
- `ui/scripts/panes.js` and `computer.html` are the first applications of this
  model, and both are provisional.

## Alternatives

- **Keep the iframe.** Simplest, and it makes ỌKỌ a page that borrows a browser.
  Rejected: it cannot browse the web it claims to browse.
- **Ship a bundled Chromium and forget Servo.** Rejected for now: no prebuilt
  aarch64 Chromium exists, and Servo is the engine that can actually be
  installed and run here (ADR 0001, #5).
- **Client-side rendering with server-side only for games.** Rejected as two
  pipelines: the second one is where the bugs and the divergence live.

## Revisit if

Frame streaming proves too expensive at the sizes people actually use, or Servo's
protocol cannot drive a live session well enough to paint one.
