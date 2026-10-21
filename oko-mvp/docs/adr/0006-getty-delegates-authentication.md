# 0006 — The getty replacement delegates authentication to login

Status: accepted · Date: 2026-10-20

## Context

ADR 0002 chose PAM, a root `oko-authd` helper and a themed HTML login screen.
That is still the right shape for a graphical console. It is not the smallest
safe first step for a barebones TTY, which is the current target:

- the console may have no browser, no display server and no GPU;
- `login` already performs authentication through PAM on Linux/FreeBSD and BSD
  auth on OpenBSD/NetBSD, plus TTY ownership, utmp and the privilege drop;
- a custom PAM helper is security-critical code and should not be rushed to
  make a machine boot.

The thing that must be replaced is getty's prompt and session choice, not the
platform's authentication stack.

## Decision

**`oko-getty` is a thin prompt; `login` authenticates; `oko-session` is the
session.**

- `oko-getty` runs as root, like getty. It opens the TTY, prints the ỌKỌ
  banner, reads one username, validates it conservatively, and execs
  `login -p <user>` with the TTY as stdin/stdout/stderr.
- It never reads a password, never links PAM, never runs setuid, and passes the
  username as one `Command` argument, never through a shell.
- `login` prompts for the password and starts the user's shell. The console
  user's shell is set to `oko-session`, which runs the unprivileged core and
  the line console. A plain shell remains available by changing the shell or
  using the fallback VT.
- `oko-session` needs no display server. It is the first launchable session;
  the display server and presenters attach to the same core later.
- A themed graphical login remains possible later through ADR 0002's `AuthPort`
  and helper. This record removes no security boundary; it defers building a
  new one.

## Alternatives

- **Full PAM helper now.** Rejected for this slice: security work, and not
  required to boot. The ADR 0002 design remains the reference for a themed
  login.
- **Auto-login appliance.** Rejected: physical access would become session
  access.
- **Setuid `oko-session`.** Rejected: a setuid binary parsing terminal input is
  the vulnerability this design exists to avoid.
- **Keep getty and run ỌKỌ from `.profile`.** Rejected: it makes ỌKỌ a shell
  startup trick, not the console session, and leaves getty's prompt in place.

## Consequences

- The login prompt is the system's text prompt, not an ỌKỌ theme. That is the
  cost of not writing an authentication stack first.
- `oko-session` must be listed in `/etc/shells` and set with `chsh`.
- The install keeps a fallback getty on another VT. A failed console must not
  lock the machine.
- `oko-getty` is root because getty is root. Its only parsed input is one
  username, with a strict character set and a length bound.
- The graphical login screen in ADR 0002 is now a later feature, not the boot
  path.

## Revisit if

A themed login is required, `/bin/login` is unavailable or unsuitable on a
target OS, or the session needs to start before `login` (for example, a
single-purpose appliance image that explicitly accepts the physical-access
risk).
