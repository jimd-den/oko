#!/usr/bin/env bash
# Run each Rust test on its own and record progress in a log file.
#
# Usage: scripts/run-tests-individually.sh [logfile]
#
# Why: a full `cargo test` runs every test of a binary in parallel threads. One
# test that loops and allocates without bound then eats all container RAM
# (~2.9 GB here) and the kernel OOM-killer kills the whole test binary, which
# also takes the terminal down with it. One test per process, with RLIMIT_AS as
# a backstop, keeps a runaway test from touching the machine.
#
# Only "RUNNING"/"ok"/"FAIL" lines are echoed; full test output goes to the log.
set -uo pipefail

cd "$(dirname "$0")/.."
LOG="${1:-test-run.log}"
CARGO="${CARGO:-cargo}"
# Extra cargo arguments, split on whitespace on purpose so a caller can pass
# several ("--offline --jobs 1") without the script guessing.
CARGO_FLAGS="${CARGO_FLAGS:-}"

run_cargo() {
    # The subcommand comes first: --jobs is a per-subcommand option, so
    # `cargo --jobs 1 test` is rejected while `cargo test --jobs 1` is not.
    local subcommand="$1"
    shift
    # shellcheck disable=SC2086
    "$CARGO" "$subcommand" $CARGO_FLAGS "$@"
}
MEM_CAP_KB="${MEM_CAP_KB:-1500000}" # address space cap per test run (KB)

: >"$LOG"

say() {
    printf '%s\n' "$*"
    printf '%s\n' "$*" >>"$LOG"
}

# Test binary names from Cargo.toml [[test]] targets.
test_binaries() {
    awk '/^\[\[test\]\]/{seen=1; next}
         seen && /^name[ \t]*=/{sub(/^name[ \t]*=[ \t]*"/,""); sub(/".*/,""); print; seen=0}' Cargo.toml
}

failed=0
total=0

say "building all test targets"
if ! run_cargo test --no-run >>"$LOG" 2>&1; then
    say "FAIL    build :: cargo test --no-run (see $LOG)"
    exit 1
fi

run_target() {
    local label="$1"
    shift
    local names name
    names="$(run_cargo test "$@" -- --list --format terse 2>>"$LOG" | sed -n 's/: test$//p')"
    if [ -z "$names" ]; then
        say "SKIP    $label :: no tests"
        return
    fi
    for name in $names; do
        total=$((total + 1))
        say "RUNNING $label :: $name"
        # `exec` cannot start a shell function, so this call spells the binary
        # and the flags out again; CARGO_FLAGS is split on purpose.
        if (ulimit -v "$MEM_CAP_KB"; exec "$CARGO" test $CARGO_FLAGS "$@" -- --exact "$name") >>"$LOG" 2>&1; then
            say "ok      $label :: $name"
        else
            say "FAIL    $label :: $name"
            failed=$((failed + 1))
        fi
    done
}

run_target "lib" --lib
run_target "bins" --bins
for bin in $(test_binaries); do
    run_target "$bin" --test "$bin"
done

if [ "$failed" -eq 0 ]; then
    say "ALL OK ($total tests)"
else
    say "$failed of $total tests FAILED"
fi
exit $((failed > 0))
