# Seam audit — the six files over the 200-line ceiling

Read-only audit. **No source or test file was edited; no cargo command was run.**
Line numbers are the file as it stands today. New-file counts are the moved block
plus an estimated preamble/import overhead (6-10 lines), so they estimate a move,
not a rewrite.

## Verification limits — read before acting

- `cargo test --offline --lib` (65 tests, ~0.3 s) compiles **blink_install.rs,
  parser.rs, host_api.rs**. Those three splits can be validated by a real gate.
- **blink.rs** is `#[cfg(feature = "blink")]`, and that feature does not compile
  today: `ChromiumEmbedder` holds `*mut c_void`, so the adapter is not
  `Send + Sync` and `impl BrowserEnginePort for ...` fails. A split there cannot
  be validated by a build until that is fixed — land the split with the fix, or
  accept inspection only.
- **servo.rs** is not declared in `src/browser/adapters/mod.rs`, so it is never
  compiled. Decide declare-or-delete before spending a cycle on its seam.
- No split below needs a `Cargo.toml` change: every new file is a submodule of an
  existing target. The flat Rust 2018 layout (`foo.rs` + `foo/bar.rs`) keeps every
  current file path unchanged; renaming to `foo/mod.rs` also works but is churn.

## 1. `src/browser/adapters/blink_install.rs` — 428 lines

**Responsibility (preamble L1-14).** Script the explicit `blink install` flow —
download → verify the checksum recorded in `toolchain/versions.lock` → build the
content embedder — and report install status. Crypto is pure Rust so the flow is
offline-testable.

**Blocks that do not serve it**

| Block | Lines | Why it is a different concern |
|---|---|---|
| `Sha256`, `sha256()`, `K` | L32-116 | A general SHA-256 primitive, not the flow. Changes with the digest algorithm or its vectors. |
| `LockEntry`, `parse_blink_entry()` | L21-30, L161-201 | The `versions.lock` file format. Changes with the lock schema. |
| downloader selection + curl/wget shell-out | L226-254, L286-295 | The imperative shell (process I/O) versus the flow logic. Changes with the download tool. |
| `fetch_checksum()` | L314-334 | Reporting a digest *for the lock record*; belongs with the lock reader. |
| `mod tests` | L337-428 | Verification, not the flow. |

**Proposed files**

| New file | Contents | Est. lines |
|---|---|---|
| `blink_install/sha256.rs` | `Sha256`, `sha256`, `K`, the 2 known-vector tests | ~115 |
| `blink_install/lock.rs` | `LockEntry`, `parse_lock_entry(&Path)`, `fetch_checksum` + its test | ~100 |
| `blink_install/download.rs` | `select_downloader`, `download_archive(url, dest)` | ~49 |
| `blink_install.rs` (kept) | state enums, `BlinkInstaller`, `new`/`status`/`install`/`build`, 3 flow tests | ~195 |

The kept file lands in the band but in the warning band; moving the 3 flow tests
to `blink_install/tests.rs` (~59) drops it to ~137. `InstallState`,
`InstallResult` and `BlinkInstaller` must stay: `src/ipc/server/mod.rs` and
`handlers/blink.rs` name them by that path.

**Move order (green after every step, gate `cargo test --offline --lib`)**

1. `Sha256` + `sha256` + `K` + their 2 tests → `sha256.rs` (pure move).
2. `LockEntry` + lock parser → `lock.rs` as
   `pub(crate) fn parse_lock_entry(path: &Path) -> Result<LockEntry, String>`
   (the path arrives as an argument, §3.2) — a signature change, its own step.
3. `fetch_checksum` + its test → `lock.rs` (uses `super::sha256::sha256`).
4. `select_downloader` + the download block → `download.rs` as
   `pub(crate) fn download_archive(url: &str, dest: &Path) -> Result<(), String>`.
5. Optional: the 3 flow tests → `blink_install/tests.rs`.

**Behaviour-preserving?** Steps 1, 3, 5 are pure relocation. Steps 2 and 4 change
an internal signature. Step 2 is pure (`Path` in, record out) and should get a
direct characterization test written **before** the move; it passes before and
after, which is what proves the extraction. Step 4 shells out to curl/wget, has no
coverage today (the install tests pre-create the archive and skip it), and cannot
be unit-tested offline — move it by inspection and say so.

## 2. `src/browser/adapters/blink.rs` — 294 lines

**Responsibility (preamble L1-16).** Implement `BrowserEnginePort` against the
Chromium embedder interface, and declare the embedder-interface boundary.

**Blocks that do not serve it.** `ChromiumEmbedder` + `mod embedder_ffi`
(L25-42, L107-147): the Chromium ABI and its opaque handle — a different reason to
change (the upstream ABI, not the port), and the exact block that must become
`Send + Sync` for the feature to compile. `mod tests` (L247-294).

**Proposed files.** `blink/ffi.rs` ~67 (handle + `extern "C"` declarations);
`blink.rs` ~188; `blink/tests.rs` ~48.

**Move order.** Handle + FFI → `ffi.rs`; then tests → `tests.rs`. There is no
green gate today, so land both moves with the `Send + Sync` fix and use the
feature build as the evidence.

**Behaviour-preserving?** Yes, both. One public path changes —
`browser::adapters::blink::ChromiumEmbedder` → `blink::ffi::ChromiumEmbedder`.
Nothing in the repo uses it. It is the only public-surface change in this audit;
record it in the commit body.

## 3. `src/browser/adapters/servo.rs` — 300 lines

**Responsibility (preamble L1-13).** Same as blink.rs: implement
`BrowserEnginePort` against the Servo embedder interface.

**Blocks that do not serve it.** `ServoEmbedder` L22-39 and `mod embedder_ffi`
L104-145 (the Servo ABI); `mod tests` L245-300.

**Proposed files.** `servo/ffi.rs` ~66; `servo.rs` ~187; `servo/tests.rs` ~56.
**Move order and behaviour.** Identical to blink.rs, with the same — worse —
verification gap, since the module is not compiled at all.

## 4. `src/ipc/json/parser.rs` — 257 lines

**Responsibility (preamble L1-2).** Turn JSON text into `JsonValue` with small
composable functions.

**Blocks that do not serve it.** Only `mod tests` (L188-257, 70 lines, 27% of the
file). Everything else is the parser.

**Proposed files.** `parser.rs` ~188 (one line added: `#[cfg(test)] mod tests;`),
`parser/tests.rs` ~70. One move; gate `cargo test --offline --lib`; pure
relocation, and `use super::*` keeps private access.

**Do not split the scanner.** `parse_value` (L20-35) dispatches into
`parse_object`/`parse_array`, which call `parse_value` back, and all of them share
`skip_ws`. Splitting scalars from containers puts that recursion across a file
boundary — a module cycle for no gain. The file is one responsibility; the test
module is the seam.

## 5. `src/browser/api/host_api.rs` — 203 lines

**Responsibility (preamble L1-4).** The typed, permission-checked host API
contract for the desktop UI: request/response vocabulary, the permission rule, and
the port traits.

**Blocks that do not serve it.** Only `mod tests` (L114-203, 90 lines, 44% of the
file — eight tests, all against the single pure function `validate_request`).

**Proposed files.** `host_api.rs` ~114; `host_api/tests.rs` ~90. One move; gate
`cargo test --offline --lib`; pure relocation.

**Rejected alternative.** Moving `capabilities_required` + `validate_request`
(L53-88) into `host_api_permissions.rs` also fits, but `capabilities_required` is
an exhaustive `match` on `HostApiRequest`: a new variant forces an edit in both
files, so they change together and stay together. Unrelated finding, not a split:
nothing outside this file uses `HostApiPort`, `validate_request` or
`TerminalSessionService` — the port has no adapter yet.

## 6. `tests/application/command_service_test.rs` — 218 lines

**Responsibility (preamble L1-4).** Exercise the `CommandService` flow — parse →
validate → handle → present — with real adapters owned inline in each test.

**Block that does not serve it.** `register_basic` (L38-104, 68 lines). It does
not exercise the flow; it *defines* the command table the flow is driven through,
and it changes when the built-in command set changes, not when a behaviour does.

**Proposed files.**
`tests/application/command_service_test/builtin_commands.rs` ~79 (`register_basic`
plus its own `use` lines); `command_service_test.rs` ~151 (preamble, helpers, the
6 tests). One move plus `mod builtin_commands;`. Gate:
`cargo test --offline --test application_command_service_test`. Pure relocation
into a submodule of the same target; access is unchanged.

**Rejected alternative.** Splitting off `execute_ui_mode_and_browser_engine`
(L186-218) is the tempting behaviour-cluster split, but that test needs
`register_basic` + `full_policy` + `make_ctx` (91 lines), so it would duplicate
them or need a second support module and a new `[[test]]` entry. Not worth it for
33 lines.

## Summary

| File | Now | After | New files |
|---|---|---|---|
| `blink_install.rs` | 428 | ~195 (or ~137 + tests) | `sha256.rs` ~115, `lock.rs` ~100, `download.rs` ~49 |
| `blink.rs` | 294 | ~188 | `ffi.rs` ~67, `tests.rs` ~48 |
| `servo.rs` | 300 | ~187 | `ffi.rs` ~66, `tests.rs` ~56 |
| `parser.rs` | 257 | ~188 | `tests.rs` ~70 |
| `host_api.rs` | 203 | ~114 | `tests.rs` ~90 |
| `command_service_test.rs` | 218 | ~151 | `builtin_commands.rs` ~79 |

**Files I would not split** beyond their test module: `parser.rs` — the scanner is
one mutually recursive unit; `host_api.rs` — the permission table must change with
the request enum; `blink.rs`/`servo.rs` — the session table is the port
implementation's own state, and extracting it needs `pub(crate)` accessors
(indirection for no gain). The ~85% textual duplication between `blink.rs` and
`servo.rs` is a real finding, but removing it needs a shared skeleton — an
abstraction, which this brief excludes.

**Recommended first move:** `blink_install.rs` → `sha256.rs`. Largest file, the
moved block carries its own tests, and the gate is a 0.3 s offline lib run.
