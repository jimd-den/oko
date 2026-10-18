# ỌKỌ build and gate entry points (AGENTS.md §4.7, §8.1).
#
# `make check` is the whole-repo gate: line budget, types, lint, tests. Run it
# before claiming anything is done; a red gate reported as green is worse than
# no gate.

CARGO ?= cargo
CARGO_FLAGS ?= --offline --jobs 1

.PHONY: check budget typecheck lint test test-engines fmt

check: budget typecheck lint test

budget:
	python3 tools/check_line_budget.py

typecheck:
	$(CARGO) check $(CARGO_FLAGS) --all-targets

lint:
	$(CARGO) clippy $(CARGO_FLAGS) --all-targets -- -D warnings

# One test per process, under an address-space cap: a runaway test must not take
# the machine with it (scripts/run-tests-individually.sh explains why).
test:
	CARGO="$(CARGO)" CARGO_FLAGS="$(CARGO_FLAGS)" scripts/run-tests-individually.sh

# Every enabled engine backend passes its own suite (SRD ENG-005, CI-004).
test-engines:
	$(CARGO) test $(CARGO_FLAGS) --lib
	$(CARGO) test $(CARGO_FLAGS) --features blink --lib
	$(CARGO) test $(CARGO_FLAGS) --features servo --lib

fmt:
	$(CARGO) fmt --check
