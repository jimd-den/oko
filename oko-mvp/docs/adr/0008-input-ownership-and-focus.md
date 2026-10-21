# 0008 — The display server owns input; focus routes it

Status: accepted · Date: 2026-10-20

## Context

A bare console has one keyboard stream. A display server may show several
surfaces: a terminal with a shell, a web page, a settings pane. If each app
opened `/dev/tty` or the keyboard device, focus would be advisory, a background
app could read a password, and a full-screen app could trap the customer.

The terminal case makes the split concrete: the terminal app has a PTY and a
shell, but the physical keyboard is not its device. The shell owns the PTY
slave's line discipline; the display server owns the physical input.

## Decision

**The display server owns the input device. Focus routes events to exactly one
surface.**

- The input source (a TTY first, evdev/wscons next) is opened by the server. It
  sets raw mode, reads bytes, and turns them into `InputEvent`s. An app never
  opens the device.
- `DisplaySession` has one focused surface. Input is written only to that
  surface's connection. An unfocused app receives nothing; input while nothing
  is focused is dropped.
- A terminal app owns its PTY, not the keyboard. It receives key events, writes
  the corresponding bytes to the PTY master, and renders the PTY grid into a
  surface. The shell sees a normal terminal on the slave side.
- The wire carries input as `ServerMessage::Input` with engine-neutral key
  codes. Printable characters use their Unicode scalar value; control keys use
  their ASCII value; terminal-only keys use the reserved 0x100 range.
- Raw mode is a guard: it saves termios and restores it on drop. Ctrl-C is a
  key in raw mode, not a signal, and the app decides what it means.
- The server must reserve a secure-attention key before the input route. That
  key is not implemented yet; until it is, a full-screen app can hold the
  console. A VT switch (Linux Ctrl-Alt-F2) remains the kernel-level escape, and
  a fallback getty on another VT is part of the install.
- The line console in `oko-session` is the fallback: when no display server is
  running, it owns stdin directly. That is the recovery path, not the normal
  path.

## Alternatives

- **Each app opens `/dev/tty`.** Rejected: no reliable focus, no isolation, and
  a background app can read keystrokes.
- **Leave the TTY in canonical mode and deliver lines.** Rejected: the server
  cannot route individual keys or implement focus, and line editing belongs to
  the focused PTY, not the physical console.
- **Use X11/Wayland input.** Rejected: those are the systems ỌKỌ replaces on a
  bare console.
- **Give the terminal app a raw device handle.** Rejected: it makes the
  terminal privileged over every other surface and breaks the focus boundary.

## Consequences

- Terminal apps must handle key sequences; the PTY's line discipline handles
  line editing for the shell after the bytes arrive.
- The server needs a stop/lock path that is not a normal key. `oko-display`
  currently has none; it is stopped with a signal. The secure-attention key is
  the next security decision, not polish.
- evdev and wscons adapters push the same `InputEvent`s into the same channel;
  the session and the wire do not change.
- Pointer input is in the vocabulary and the wire, but no pointer source is
  wired yet.

## Revisit if

Multiple seats or a lock screen need input grabs, or a target OS has no usable
raw TTY path and requires a device-specific input adapter.
