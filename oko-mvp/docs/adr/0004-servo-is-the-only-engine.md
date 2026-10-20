# 0004 — Servo is the only browser engine

Status: accepted · Date: 2026-10-20

## Context

The previous technology strategy named Blink + V8 + Skia as the primary stack,
with Servo as an experimental adapter. That strategy conflicts with three
constraints that now govern the project:

- the app must stay minimal: one engine built, secured, updated and tested;
- the machine this runs on is linux-aarch64, where no prebuilt Chromium exists
  but a pinned Servo release does (ADR 0003);
- the repository still carried a Blink adapter, a Blink installer, a second
  SHA-256 implementation path and three `blink.*` IPC routes that no shipped
  configuration could use.

Keeping both stacks meant every build, capability table and test matrix carried
a second engine that was not the product.

## Decision

**Servo is the only browser engine ỌKỌ ships.**

- The Servo adapter is compiled unconditionally. There is no `blink` or `servo`
  feature to remember.
- `mock` remains a dev stub used by tests. It is visibly experimental and is
  never a fallback for an unavailable Servo.
- The engine is still driven as a separate process, installed from the pinned
  release in `toolchain/versions.lock` and located under
  `third_party/engines/servo/`.
- `select` reports an unknown engine name as unavailable. It never substitutes
  Servo or mock.
- The Blink adapter, Blink installer, `blink.*` routes and their host-API
  vocabulary are removed. The digest code they owned moved to
  `browser::adapters::engine_binary::sha256`, where the remaining installer
  uses it.
- A browser instance still uses exactly one engine backend (SRD ENG-001).

## Alternatives

- **Keep Blink as the primary and Servo as an option.** Rejected: it doubles
  the engine build, security and test surface for a binary this platform cannot
  build, and the user asked for Servo.
- **Keep Blink code behind a feature flag "for later".** Rejected: dead code in
  a capability table is still a claim the product makes. Removing it is the
  minimal honest state; it is recoverable from git if a Chromium embedder ever
  becomes viable.
- **Delete the mock adapter too.** Rejected: the core and its tests must run
  without an installed engine, and a declared dev stub is cheaper than making
  every test depend on a 133 MB binary.

## Consequences

- `make check` exercises the Servo adapter on every run; there is no second
  engine suite.
- The default profile selects `servo`. A missing binary produces an unavailable
  engine and the terminal core still starts (SRD ENG-008, §4.2).
- The `experimental` flag on Servo stays true. That is upstream's status, not a
  statement that another engine is hiding behind it.
- Adding a second engine later is a new decision record, a new adapter and a new
  build configuration — not a flag flipped on existing code.

## Revisit if

A supported Chromium embedder becomes available for a target platform and the
one-engine constraint stops buying more than it costs.
