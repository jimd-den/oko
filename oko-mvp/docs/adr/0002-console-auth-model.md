# 0002 — The console authenticates through PAM, and the login screen does not

Status: accepted · Date: 2026-10-19 · Issue: #4

## Context

The boot vision has ỌKỌ take tty1 from `getty`. The getty prompt is not text on
a screen, it is a **security boundary**: whatever replaces it owns
authentication for that machine. Issue #3 is blocked on this decision on
purpose, because retrofitting auth behind a boot service is how a console ends
up with a session that nobody authenticated.

Two requirements pull against each other: people should be able to **design the
login screen**, and the console must not **introduce a vulnerability**. Design
freedom and a security boundary are not the same thing, and conflating them is
the vulnerability.

Verified on this machine: `/lib/aarch64-linux-gnu/libpam.so.0` is present
(0.85.1), `/etc/pam.d/login` is the standard Shadow config, and `su` and `login`
are root-owned. There is **no** `pam_appl.h`, so the build needs `libpam0g-dev`
for headers, or declares the handful of `pam_*` symbols itself the way this
crate already declares other FFI.

## Decision

**PAM, with a hard split between policy, privilege, and presentation.**

### Policy — native, unthemeable, decided by the host

A new `AuthPort`: `authenticate(credentials) -> Result<Session, AuthError>`.
The adapter is `PamAuth`, authenticating against a PAM service file (`oko`, a
copy of `login`'s stack). Selected by the profile, not by the screen:
`auth = "pam"`.

### Privilege — one small helper, and the console never has it

This is the part that decides whether the change is safe. `pam_unix` reads
`/etc/shadow`, so **verification needs privilege**, and establishing a session
(`pam_open_session`, the session's own PAM stack) needs it too. That is exactly
why `login` and `su` are root-owned.

So the privileged surface is one helper with a narrow contract, not the UI:

- `oko-authd` runs as root, owns a unix socket, and does exactly two things:
  `pam_authenticate` for a named user with a supplied secret, then
  `pam_open_session`. It reports success or failure and nothing else.
- It checks `SO_PEERCRED` on every request and accepts only the console's uid.
- It never returns PAM text, module errors or timing detail beyond a
  constant-time failure, so it cannot be used as a user-enumeration oracle.
- **`oko-host` and the console stay unprivileged** (SEC-001: the host is not to
  run as root). A setuid `oko-console` is explicitly rejected — a setuid binary
  that parses untrusted input is the vulnerability this decision exists to
  avoid. A root systemd service behind a peer-credential-checked socket has no
  setuid bit to get wrong.

### Presentation — a theme, and it cannot say yes

`ui/themes/login/*` holds markup, CSS and assets. A designer may restyle the
gate completely. A theme **cannot**: choose the adapter, change the policy, read
a credential after submission, or grant a capability. Same rule as CFG-008 for
desktop themes, applied to the one screen where getting it wrong is worst.

The screen posts credentials to the host over the same typed, versioned API as
everything else; the host asks the helper and answers. The screen is a Humble
Object: it renders and it submits, and it is never the thing that decides.

## Threat model, written down

- **Physical access is no longer session access.** That is the whole point of
  choosing PAM over appliance mode, and it is the property #3 must not lose.
- **The console must not leak what it handles:** no password echo, no credentials
  in logs or crash reports (SEC-007), a lock screen with a defined
  secure-attention path, and the same failure delay `login` applies
  (`pam_faildelay`, 3 s in the shipped config).
- **The helper is the new attack surface.** It is small, root, socket-only, and
  peer-credential-checked; it is also the first thing to fuzz and the first
  thing to review.
- **A failed console start falls back to getty on tty1.** A console that cannot
  start must never leave a machine with no way in.

## Alternatives

- **A, appliance only (auto-login).** Rejected by the owner: physical access
  would be session access, and full-disk encryption would be the only boundary.
- **B, PAM by default with appliance as a configured mode.** Kept as a profile
  value, not the default, so the boundary is never quietly absent.
- **A theme that checks the password in JavaScript.** Rejected outright: that is
  a login screen that can be edited into one that always says yes.
- **Setuid console.** Rejected above; privilege lives in the helper or nowhere.
- **Authenticate without PAM** (read `/etc/shadow` ourselves, check hashes).
  Rejected: that is writing an authentication stack, which is the same class of
  mistake as hand-rolling a VT parser, with worse consequences.

## Consequences

- #3 (the boot service) now needs two units, not one: the unprivileged console
  and the privileged helper, with the socket's ownership as part of the install
  procedure.
- #2 (evdev input) is narrower: input is read by the console process, and the
  console's identity is no longer a profile guess — it is whatever the user
  authenticated as.
- A themed login screen needs a way to be previewed without booting into it, or
  designers will test on the real gate.
- `libpam0g-dev` becomes a build dependency, or the `pam_*` FFI declarations are
  written by hand. Either way it is recorded in `toolchain/versions.lock`.

## Revisit if

The console is ever exposed beyond the physical machine, or a second human user
needs a session on it.
