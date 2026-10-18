# split-blink-install — verified by the parent, after repair

The `split-blink-install` subagent created `src/browser/adapters/blink_install/`
and exited **without a reply and with a non-compiling tree**. The parent
verified the work by running the gate rather than trusting a summary, found four
defects, and repaired them. Recorded here because a child's claim is not
evidence, and a silent exit is the least trustworthy case of all.

## What the child left

| Defect | Evidence |
|---|---|
| Tree did not compile | `error: visibility `pub(super)` is not followed by an item` (sha256.rs:7); `error: `self` parameter is only allowed in associated functions` (download.rs:10) |
| Four tests stopped running | `tests.rs` existed but `mod.rs` never declared it, so lib tests dropped 76 -> 72: `status_not_installed_by_default`, `install_without_checksum_fails_cleanly`, `install_creates_built_state`, `fetch_checksum_reports_digest` |
| Duplicated statements | `let _ = fs::create_dir_all(&self.downloads_dir); let archive = ...;` twice in a row in `mod.rs` |
| Dead fields | `LockEntry`'s `component`, `upstream_project`, `license` were parsed and never read |

## Repairs made by the parent

- `pub(super) /// doc` -> doc comment then `pub(super) struct`.
- `fn select_downloader(&self)` -> `fn select_downloader()` (a free function in a
  sibling module has no `self`), and its call site.
- `#[cfg(test)] mod tests;` added; the test module body dedented.
- Duplicate statements removed.
- Unread `LockEntry` fields removed: the rest of the `[blink]` section is the
  inventory the release tooling reads from `toolchain/versions.lock` itself, so
  parsing it here duplicated a source of truth.

## Verification (parent, after repair)

    cargo build --offline --jobs 1                  -> clean, no warnings
    cargo test  --offline --jobs 1 --lib            -> 76 passed, 0 failed
    python3 tools/check_line_budget.py              -> 0 over the ceiling

76 is the count before the split plus the four capability tests added in the
same session, so no test was lost. Sizes: mod.rs 168, sha256.rs 115, lock.rs 59,
download.rs 44, tests.rs 71 - all inside the 25-200 band.

Behaviour equivalence was checked mechanically: every non-comment, non-blank
line of the original 428-line file is present in the package, except the 24 that
the split deliberately renamed (visibility, the path-taking lock parser, the
moved download block, the removed dead fields).
