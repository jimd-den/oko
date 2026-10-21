# Booting into ỌKỌ

The first target is a bare TTY: no X11, no Wayland, no display server. `login`
owns authentication; `oko-session` is the unprivileged session that runs after
it. `oko-getty` only reads a username and hands it to `login`.

## Install

1. Build and install the binaries:

       cargo build --release --bin oko-getty --bin oko-session
       install -m 0755 target/release/oko-getty /usr/local/bin/oko-getty
       install -m 0755 target/release/oko-session /usr/local/bin/oko-session

2. Add the session to the list of valid shells:

       echo /usr/local/bin/oko-session >> /etc/shells

3. Set it as the shell for the console user:

       chsh -s /usr/local/bin/oko-session <user>

4. Replace getty on one console, and keep a fallback getty on another.

## Linux (systemd)

Override the getty unit for `tty1`:

    install -d /etc/systemd/system/getty@tty1.service.d
    install -m 0644 packaging/linux/oko-getty.conf \
      /etc/systemd/system/getty@tty1.service.d/oko-getty.conf
    systemctl daemon-reload
    systemctl restart getty@tty1

Keep `getty@tty2` untouched. If `oko-getty` or `oko-session` fails, switch to
tty2 with Ctrl-Alt-F2 and repair.

## FreeBSD

Edit `/etc/ttys` using `packaging/freebsd/ttys.example`. Keep `ttyv1` on the
system getty. Then `kill -HUP 1`.

## OpenBSD / NetBSD

Edit `/etc/ttys` using `packaging/openbsd/ttys.example`. Keep `ttyC1` on the
system getty. Then `kill -HUP 1`.

## Safety

- `oko-getty` runs as root because getty does; it never reads a password.
- Do not set a setuid bit on `oko-session` or `oko-getty`.
- Test in a VM first. A console that cannot start must not be the only way in.
- `oko-session` starts the core and a line console today. The display server
  and presenters are separate; they are not required for login.
