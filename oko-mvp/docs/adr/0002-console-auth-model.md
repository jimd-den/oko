# 0002 — The console owns authentication, and the login screen does not

Status: accepted · Date: 2026-10-19 · Issue: #4

## Context

The boot vision has ỌKỌ take tty1 from `getty`. The getty prompt is not text on
a screen, it is a **security boundary**: whatever replaces it owns
authentication for that machine. Issue #3 is blocked on this decision on
purpose, because retrofitting auth behind a boot service is how a console ends
up with a session that nobody authenticated.

The requirement pulling the other way: people should be able to design the login
screen. Design freedom and a security boundary are not the same thing, and
conflating them is the vulnerability.

## Decision

**Hybrid (option C), with a hard split between policy and presentation.**

- **Policy is native and unthemeable.** A new `AuthPort`:
  `authenticate(credentials) -> Result<Session, AuthError>`. Two adapters:
  `ApplianceAuth` (fixed user, no password at the glass) and `PamAuth`
  (authenticates exactly as `login` does). Selected by the profile:
  `auth = "appliance" | "pam"`. Default `appliance`, because the console is a
  personal machine's console and full-disk encryption is the real boundary.
- **Presentation is a theme.** `ui/themes/login/*` holds markup, CSS and assets.
  A designer may restyle the gate completely. A theme **cannot**: choose the
  adapter, change the policy, read a credential after submission, or grant a
  capability. Same rule as CFG-008 for desktop themes, applied to the one screen
  where getting it wrong is worst.
- **The host decides, always.** The login screen posts credentials to the host
  over the same typed, versioned API as everything else; the host authenticates
  and answers. The screen is a Humble Object: it renders and it submits, and it
  is never the thing that says yes.

## Threat model, written down

- **Appliance mode:** physical access is session access. Anyone at the keyboard
  is in. The boundary is full-disk encryption, not the console. This is stated
  in the profile's documentation, not implied.
- **PAM mode:** the console must not leak what it handles — no password echo, no
  credentials in logs or crash reports (SEC-007), a lock screen with a defined
  secure-attention path, and the same rate limiting `login` applies.
- **Both:** a failed console start falls back to getty on tty1. A console that
  cannot start must never leave a machine with no way in.

## Alternatives

- **A, appliance only.** Simplest, and honest about physical access, but it makes
  a multi-user machine impossible and the decision irreversible without rework.
- **B, PAM only.** Keeps the boundary where users expect it, but puts PAM and
  secure-attention work in the path of the first boot for a machine that is
  mostly one person's.
- **A theme that renders the password field and checks it in JavaScript.**
  Rejected outright: that is a login screen that can be edited into a login
  screen that always says yes.

## Consequences

- #3 (the boot service) can proceed once `AuthPort` exists; it must know who the
  console runs as, which is now a profile value rather than a hardcoded user.
- #2 (evdev input) still needs a permissions decision, and it is now narrower:
  input is read by the console process, whose identity the profile fixes.
- A themed login screen needs a way to be previewed without booting into it, or
  designers will test on the real gate.

## Revisit if

The machine gains other human users, or the console is ever exposed beyond the
physical machine.
