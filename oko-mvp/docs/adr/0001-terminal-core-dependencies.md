# 0001 — Two crates for the terminal core, and what that costs the offline rule

Status: accepted · Date: 2026-10-19 · Issue: #5

## Context

`TerminalPort` has been served by a stub: `LinuxPtyAdapter` buffers a line and
runs it through `sh -c`. There is no VT parser, no cell grid, no scrollback, no
selection, so TERM-004 (a defined terminal compatibility profile) cannot be met
by that adapter, and hand-writing a VTE parser is the swamp the ecosystem
already drained.

Cargo.toml, the module preambles and SRD AT-019 all say the core has **zero
external crates** so it builds and tests offline. That invariant has now been
spent, deliberately, and this record is the receipt.

## Decision

Add two pinned crates behind the ports that already exist:

- `alacritty_terminal` 0.26 — UI-independent terminal core: VTE parser, cell
  grid, scrollback, selection. Feeds on PTY bytes, renders anywhere: the Servo
  DOM today, a DRM framebuffer tomorrow.
- `portable-pty` 0.8 — the real PTY adapter behind `TerminalPort`. The port
  trait does not change; only the adapter behind it does.

Both are recorded in `toolchain/versions.lock` with exact versions (DEP-001).

## Alternatives

- **Keep hand-rolling.** Rejected: a correct VT parser is years of edge cases,
  and the console is the product.
- **Write a VT parser from the specification.** Rejected for the same reason,
  plus it would be untestable against the real world without a corpus.
- **Vendor the dependency tree into `vendor/`.** Not done yet. The registry
  cache makes `cargo build --offline` work on this machine, which is what the
  gate needs; a fresh clone needs the network once. Vendoring is the follow-up
  if a truly air-gapped build is required.

## Consequences

- **AT-019 changes meaning.** It said "core tests run without Internet access"
  and enforced it by having no dependencies at all. It now means what it says:
  `cargo test --offline` passes against a warm cache, and `make check` still
  runs offline. That is weaker than before, and the weakening is the price.
- The dependency tree is 86 crates in `Cargo.lock`. Every one is now part of the
  supply chain that DEP-004 and DEP-005 have to track.
- `alacritty_terminal` 0.24 does **not** build on Rust 1.99 (its `nix` pin
  predates the `AsFd` bound); 0.26 does. The version is pinned for that reason.

## Revisit if

A vendored, air-gapped build becomes a requirement, or either crate stops being
maintained upstream.
