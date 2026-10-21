# 0007 — The surface wire format is a bounded header and raw RGBA

Status: accepted · Date: 2026-10-20

## Context

The display core has typed messages (`Create`, `Commit`, `Destroy`) and a
compositor. An app in another process needs to send them over a socket. The
pixels are the hard part: a 1280x800 RGBA frame is about 4 MB, and JSON with a
base64 payload adds a third to it and allocates a string of the same size.

The first process boundary also needs a hard bound: a peer must not be able to
make the host allocate an arbitrary buffer by claiming a huge frame.

## Decision

**A fixed 26-byte header followed by a raw payload.**

    magic "OKOD" | version u8 | kind u8 | id u64 | width u32 | height u32
    | payload_len u32 | payload

- Control messages have an empty payload. A `Commit` payload is exactly
  `width * height * 4` bytes of RGBA8888, top to bottom.
- The version is checked. Unknown kinds and bad magic are refused.
- `payload_len` over 32 MiB is refused before allocation.
- The session still checks surface dimensions, ownership and frame size; the
  wire bound is not a substitute for those checks.
- One connection is served at a time for now. Surfaces outlive a connection,
  which is what makes a UI reload cheap.
- Input frames exist in the vocabulary but no input pump is wired yet.

## Alternatives

- **JSON plus base64.** Rejected: a third more bytes, a string allocation of
  the frame size, and no simpler to test.
- **Shared memory first (`memfd`/`shm`).** Rejected for v1: more platform and
  lifetime surface than the first slice needs. It is the next step when frame
  rate matters.
- **Use an existing display protocol (Wayland, X11, DBus).** Rejected: those
  are the systems ỌKỌ is replacing on a bare console, and the app contract is
  deliberately smaller.
- **Write frames to files and poll.** Rejected: file lifetime, permissions and
  cleanup are worse than a socket, and there is no back-pressure.

## Consequences

- A full frame per commit costs 4 MB at 1280x800. Fine on loopback for a first
  surface; not enough for 60 fps. Damage rectangles and shared memory are the
  next performance decisions.
- A slow client blocks the server's accept loop until it disconnects. One
  connection at a time is a deliberate limit; multi-client scheduling is later.
- The 32 MiB bound allows a 2048x2048 RGBA frame, enough for the presenters
  that exist and small enough to refuse a hostile allocation.
- The terminal presenter homes the cursor and writes no trailing newline, so a
  live server overwrites its frame instead of scrolling.

## Revisit if

Multiple apps need to commit concurrently, or a frame rate above a few per
second makes a full RGBA copy the bottleneck.
