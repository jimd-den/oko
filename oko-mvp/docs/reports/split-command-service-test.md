# Split report — tests/application/command_service_test.rs (218 → 154 + 77)

Refactor only: same tests, same selection, green before and after, no behaviour
change. No test body, name, assertion or expected value was touched.

## The seam

Per docs/reports/seam-audit.md §6: `register_basic` (67 lines) does not exercise
the parse -> validate -> handle -> present flow; it defines the command table the
flow is driven through, and it changes when the built-in command set changes —
a different reason to change than the six flow tests. Moved to
`tests/application/builtin_commands.rs` (77 lines: preamble + 2 imports + the
function, byte-identical).

The six call sites keep the bare `register_basic(&mut svc);` — the parent
re-imports it with `use self::builtin_commands::register_basic;`, so the test
bodies are untouched. Verified mechanically: reversing the split (delete the
mod/use lines, reinsert the function) reproduces the original 218-line file
byte-for-byte.

## Correction to the audit's proposed path

The audit proposed `tests/application/command_service_test/builtin_commands.rs`,
citing the flat Rust 2018 layout `foo.rs + foo/bar.rs`. That subdirectory rule
applies to non-root module files. A crate root (an integration test file) is
searched in its own directory: `mod builtin_commands;` resolves to
`tests/application/builtin_commands.rs`. rustc's own error message proves it:

    error[E0583]: file not found for module `builtin_commands`
      --> tests/application/command_service_test.rs:15:1
    = help: to create the module `builtin_commands`, create file
            "tests/application/builtin_commands.rs" or
            "tests/application/builtin_commands/mod.rs"

Moved there; the plain `mod builtin_commands;` needs no `#[path]` attribute.

## Cargo.toml: no change needed

The parent brief expected a `[[test]]` entry for the new file. It is not a new
target — it is a submodule of the existing `application_command_service_test`
target, compiled only because the crate root declares it. No `[[test]]` entry,
no `#[path]`, Cargo.toml untouched. (The `M oko-mvp/Cargo.toml` in git status
is a sibling agent's pre-existing addition of the at005 / meta_line_budget
targets, present before this task started; verified via git diff.)

## Test count — unchanged (6 before, 6 after)

    $ cargo test --offline --jobs 1 --test application_command_service_test -- --list
    execute_app_launch_then_list_then_stop: test
    execute_denied_without_capability: test
    execute_help_returns_help_text: test
    execute_status_returns_running: test
    execute_ui_mode_and_browser_engine: test
    execute_unknown_command_errors: test

    6 tests, 0 benchmarks

## Line counts

| File | Before | After |
|---|---|---|
| tests/application/command_service_test.rs | 218 | 154 |
| tests/application/builtin_commands.rs | — | 77 |

Both files land in the 25-200 band, both under the 200 ceiling.

## Gate

Baseline before the change: same target, same 6 tests, green (0.6 s).

First gate attempts were blocked by a sibling agent's in-flight blink_install
split (`src/browser/adapters/blink_install/mod.rs:167` unclosed delimiter —
the lib did not compile; src/ is outside my scope, so I waited and retried).
Once the sibling landed their fix, the gate passed:

    $ cargo test --offline --jobs 1 --test application_command_service_test -- --list
    ... 6 tests, 0 benchmarks

    $ cargo test --offline --jobs 1 --test application_command_service_test
    running 6 tests
    test execute_denied_without_capability ... ok
    test execute_help_returns_help_text ... ok
    test execute_ui_mode_and_browser_engine ... ok
    test execute_unknown_command_errors ... ok
    test execute_status_returns_running ... ok
    test execute_app_launch_then_list_then_stop ... ok

    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

    $ wc -l tests/application/command_service_test.rs tests/application/builtin_commands.rs
     154 tests/application/command_service_test.rs
      77 tests/application/builtin_commands.rs
     231 total

All commands exit 0. Nothing committed to git.
