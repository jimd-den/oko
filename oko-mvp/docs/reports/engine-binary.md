# engine-binary — completed by the parent after the subagent exited silently

The `engine-binary` subagent exited with status *completed*, **no reply**, and
left the package as a skeleton: every function body was `todo!()`, including the
ones its own tests called. A `todo!()` in a code path the host reaches is a
panic waiting for a user, so this is not a "finish it later" state.

Recorded here for the same reason as `split-blink-install.md`: a child's silence
is the least trustworthy signal there is, and the brief asking for pasted
command output did not prevent it.

## What the child left

| Defect | Evidence |
|---|---|
| Every leaf unimplemented | `todo!()` in `lock.rs` (`file_name`, `tar_mode`, `parse_servo_entry`), `verify.rs`, `extract.rs`, `download.rs`, `launch.rs` |
| Tree did not compile | `mod tests;` declared, but `tests.rs` was written into a stray `lock/` directory: `failed to resolve mod tests` |
| Test import wrong | `use super::super::{parse_servo_entry, ArchiveKind}` - that path is `adapters::`, where neither name exists |
| No lock entry | `toolchain/versions.lock` had no `[servo]` section, so install had nothing pinned to fetch |
| Unreachable code | nothing called `install`, `locate` or `spawn_session`, so the package warned as dead code |

## What the parent wrote

- `lock.rs`: the `[servo]` reader, the archive container, the cache file name and
  the `tar` mode. Refuses an unset checksum and an unknown container.
- `verify.rs`: `is_unset` and `verify`, reusing the crate's existing pure-Rust
  SHA-256 rather than adding a second implementation.
- `extract.rs`: unpack with the system `tar`, stripping the single top-level
  directory the upstream archive wraps everything in.
- `download.rs`: curl, then wget; a missing downloader is a clear failure.
- `launch.rs`: the argv for both real use cases, spawn, wait, stop.
- `toolchain/versions.lock`: the pinned Servo v0.7.0 linux-aarch64 asset and its
  published digest.
- `src/cli.rs`: `--install-engine` and `--open-engine`, wired to the package.
- Tests: the lock parser (the child's, kept), plus digest verification including
  the case-insensitive path, an unset checksum, a missing archive, and the exact
  argv shapes.

## Verification (parent)

    cargo build --offline --jobs 1                    -> clean, no warnings
    cargo test  --offline --jobs 1 --lib              -> 99 passed, 0 failed
    make check                                        -> budget OK, clippy clean, 140 tests ALL OK
    python3 tools/check_line_budget.py                -> 0 over the ceiling

Then the real thing, on this machine:

    oko-host --install-engine --engine servo
      downloaded 45.9 MB, digest matched the pinned value, engine ready:
      third_party/engines/servo/servoshell

    oko-host --transport http --port 8099 --engine servo --open-engine
      ui: http://127.0.0.1:8099/
      engine process: .../servoshell (pid 63043)
      ps: servoshell http://127.0.0.1:8099/   RSS 324 MB
      after the host exits: no orphaned engine process

The engine process is stopped on the normal exit path. A SIGKILL of the host
still orphans it, because std offers no signal handling; that hazard is written
down in `launch.rs` rather than hidden.
