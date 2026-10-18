# Split report — `ipc/json/parser.rs` and `browser/api/host_api.rs`

Refactor only. No behaviour change, no test added, removed, renamed or weakened.
Files touched: exactly the four paths in scope. No `Cargo.toml` edit; no commit.

## Seam

The seam the audit proposed is correct and is what I used: both files keep their
whole production body and move only their `#[cfg(test)] mod tests` block to a
sibling module. Both test modules already `use super::*`, so a child module still
reaches the same private items — the move needs no visibility change and no
re-export.

The two rejected alternatives in the audit stay rejected, for the same reasons:

- **parser**: `parse_value` → `parse_object`/`parse_array` → `parse_value` recurses
  and all share `skip_ws`. Splitting scalars from containers puts that recursion
  across a file boundary — a module cycle for no gain. The scanner is one
  responsibility; the test module is the seam.
- **host_api**: `capabilities_required` is an exhaustive `match` on
  `HostApiRequest`, so a new variant forces an edit in both files. They change
  together and stay together.

Rust 2018 flat layout (`foo.rs` + `foo/tests.rs`) is used, so `mod.rs` files need
no edit and the existing module paths are unchanged. Test paths gain one segment
(`ipc::json::parser::tests::*`, `browser::api::host_api::tests::*`); test names
are untouched.

## Before / after line counts (`wc -l`)

| File | Before | After |
|---|---|---|
| `src/ipc/json/parser.rs` | 257 | **189** |
| `src/ipc/json/parser/tests.rs` | — | 72 |
| `src/browser/api/host_api.rs` | 203 | **115** |
| `src/browser/api/host_api/tests.rs` | — | 92 |

New files are above the ~25-line cohesion floor and below the 200-line ceiling.

## Test identity — the proof that nothing was rewritten

Every moved test line is byte-identical to the original modulo a uniform 4-space
dedent (the block lost one nesting level). Checked mechanically against
`git show HEAD:<file>`:

```
67 == 67  parser body lines, reindent(new) == original
87 == 87  host_api body lines, reindent(new) == original
EXACT MATCH: every moved test line is byte-identical modulo the 4-space dedent
```

The `git diff` of the two production files has exactly two hunks and two added
lines:

```
oko-mvp/src/browser/api/host_api.rs | 90 +------------------------------------
oko-mvp/src/ipc/json/parser.rs      | 70 +----------------------------
2 files changed, 2 insertions(+), 158 deletions(-)

+mod tests;      (host_api.rs)
+mod tests;      (parser.rs)
```

`#[cfg(test)]` is unchanged context in both, so the only production-file change is
the module declaration.

## Commands and output

Baseline recorded before any edit:

```
$ cargo test --offline --jobs 1 --lib
test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
EXIT=0
```

Hazard seen on the first baseline run (before any edit of mine): a concurrent
agent's test run shared the temp dir `oko-render-test` and made
`render::tests::handle_capture_writes_file` fail once with `assertion failed:
out.exists()`. Re-running alone gave 72/72. The shared temp dir is a flake source
for concurrent runs; it is outside this task's scope.

After the split:

```
$ cargo build --offline --jobs 1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.47s
EXIT=0
```

No warnings emitted (the only other line was `Blocking waiting for file lock on
build directory`, from a concurrent agent).

```
$ cargo test --offline --jobs 1 --lib
test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
EXIT=0
```

Same count as baseline (72), so every previously-collected lib test is still
collected and green. The moved tests are present under their new paths:

```
$ cargo test --offline --jobs 1 --lib -- --list | grep -E 'parser::tests|host_api::tests'
browser::api::host_api::tests::validate_app_launch_requires_process_spawn_cap
browser::api::host_api::tests::validate_blink_install_requires_system_configure
browser::api::host_api::tests::validate_blink_status_needs_no_capability
browser::api::host_api::tests::validate_browser_open_requires_browser_navigate_cap
browser::api::host_api::tests::validate_render_demo_requires_graphics_create
browser::api::host_api::tests::validate_status_needs_no_capability
browser::api::host_api::tests::validate_terminal_create_requires_terminal_create_cap
browser::api::host_api::tests::validate_ui_mode_desktop_requires_system_configure
ipc::json::parser::tests::parse_array_of_numbers
ipc::json::parser::tests::parse_bool_false
ipc::json::parser::tests::parse_bool_true
ipc::json::parser::tests::parse_empty_array
ipc::json::parser::tests::parse_empty_object
ipc::json::parser::tests::parse_nested_object
ipc::json::parser::tests::parse_null
ipc::json::parser::tests::parse_number_value
ipc::json::parser::tests::parse_rejects_bad_input
ipc::json::parser::tests::parse_string_value
ipc::json::parser::tests::parse_string_with_escape
```

18 tests moved (10 parser, 8 host_api).

## Left over / deviations

- Nothing is left over the 200-line ceiling among the four files.
- **Deviation from the audit, reported:** the audit said `parser.rs` lands at ~188
  and `host_api.rs` at ~114. I land at 189 and 115 because I kept `#[cfg(test)]`
  on its own line (it was already on its own line) rather than joining it to
  `mod tests;`. No line was shuffled to hit a number.
- No comment was deleted, no unrelated line reformatted, no test touched.
- `cargo build` proves the non-test code; it does not compile `#[cfg(test)]`
  modules, so the test-module compile is proved by `cargo test --lib`.
