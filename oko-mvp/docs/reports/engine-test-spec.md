# Engine test spec — browser-engine boundary and conformance matrix

Read-only survey (explorer + architect). No cargo command was run: no build, no
test. Every claim about current behaviour cites file:line.

Snapshot 2026-10-17 21:17 MDT, sha256[0:12] of what was read:
`src/main.rs` a1bb97347b46 · `src/ipc/server/mod.rs` 774bcef793a6 ·
`src/ipc/server/handlers/browser.rs` 797a38f5ae6e ·
`src/browser/adapters/mod.rs` bb5ea90eba48 · `servo.rs` 5f40b194f2f4 ·
`Cargo.toml` 76435ae1fd40 · `tests/acceptance/at008_engine_lifecycle.rs` 01c53b17fc48.

Hazard: the parent session edited `src/ipc/server/mod.rs` (21:16:24) and
`handlers/browser.rs` (21:16:28) while this survey ran. §2 gives both states.
Nothing below is verified by execution.

## 1 Where the engine is chosen, reported, torn down

| Concern | Location |
|---|---|
| Concrete adapter chosen (the only place) | `src/main.rs:28` `Box::new(MockBrowserEngineAdapter::new())` |
| Alternative adapters | `Cargo.toml:32-34` features `blink`, `servo`; gate at `src/browser/adapters/mod.rs:7-8` |
| Servo module | NOT DECLARED: `adapters/mod.rs` (12 lines) has no `servo` entry, so `--features servo` compiles nothing and `servo.rs` is dead source |
| Engine injected into the host | `src/main.rs:45` -> `IpcServer::new_stdio` -> `mod.rs:98` |
| Identity reported | `handlers/browser.rs:31-45`, route `host.browser.engine` (engine, version, experimental, state, reason) |
| Capabilities reported | `handlers/browser.rs:47-54`, route `host.browser.capabilities` |
| Route table | `dispatch.rs:44-46` |
| Initialize | `mod.rs:62-65`, once, before the message loop |
| Shutdown | `mod.rs:86-87`, best effort |
| UI consumer | `ui/scripts/main.js:157-162` (engine name + "(experimental)"; "unavailable" on throw) |
| Config field naming an engine | `src/domain/config.rs:23,49` (`browser.engine`, default `"blink"`) — no caller reads it |

## 2 An unavailable engine: the trace, and whether the core survives

Committed HEAD `37e823e`, `git show HEAD:oko-mvp/src/ipc/server.rs:65`:

    self.browser.initialize().map_err(|e| format!("{:?}", e))?;

The `?` returns from `run()` before the message loop, so `main.rs:52-55` prints
`ipc server stopped: ...` and exits 1. The terminal core never starts: no
`host.status`, no `host.terminal.create`, no session. That breaks ENG-004,
ENG-008, SRD §4.2 ("a failing optional service cannot prevent the core terminal
from starting") and §17.2 ("recovery startup must not wait for browser init").

Working tree at the snapshot (`mod.rs:62-65`, `:86-87`; `browser.rs:14-16`,
`:37-43`): the initialize failure is stored in `engine_unavailable:
Option<String>` (`mod.rs:29`), the loop still runs, `shutdown()` is best effort,
`host.browser.engine` answers `state: "unavailable"` with the reason, and
`host.browser.open` fails without touching the engine. This is the shape
ENG-004/ENG-008/§4.2 ask for. It is UNVERIFIED: the two tests in
`tests/acceptance/at008_engine_lifecycle.rs` were never run — `test-run.log`
(20:47) and `test-run-now.log` (21:13) predate the file (21:16) and list 90
tests, none from at008.

Missing entirely: the crash path. `run()` records only an `initialize()`
failure, so a crash after a good initialize (mock `simulate_crash`, `mock.rs:43`)
is never observed, and `last_crash_reason()` (`ports/engine.rs:70`) has no IPC
route, so OBS-003's crash reason is unreachable from the UI.

## 3 What the servo adapter does, and what it cannot do

Implements (`src/browser/adapters/servo.rs`): all 11 port methods (`:147-243`);
`engine_name "servo"` `:170`, `engine_version "servo-pinned"` `:174`,
`is_experimental() == true` `:179`, capabilities `:183-190`; an unlinked-embedder
guard `:92-101`; crash simulation `:84-90`; a session map `:52`; FFI symbol
declarations behind `#[cfg(feature = "servo")]` `:104-145`; five unit tests
`:245-299` that never compile (see §1).

Cannot do until linked to a real Servo build: `initialize()` always errors while
`embedder.linked == false` `:152-160`; `create_session` / `navigate` /
`execute_script` refuse `:196,:213,:234`; `execute_script` returns
`ScriptFailed` even when linked `:235-237`; the FFI block calls nothing — it only
declares symbols `:114-144`. UNKNOWN and not verifiable here: whether real Servo
exports those symbol names, and Servo's script compatibility (SRD §5.4).

How it must surface over IPC: `host.browser.engine` names `"servo"` with
`state: "unavailable"`, `experimental: true`, never a substituted engine name
(ENG-008, ENG-009). The route does exactly that for the unlinked case
(`browser.rs:37-43`) — once `servo` is declared in `adapters/mod.rs`, which it
is not.

## 4 Conformance matrix

Verdicts are against the tree at the snapshot; "unrun" means no log or command
covers it.

| ID | Test name | File | Arrange / Act / Assert | Verdict today |
|---|---|---|---|---|
| ENG-001 | `one_engine_name_is_reported_for_a_browser_instance` | `tests/acceptance/at008_engine_lifecycle.rs` | arrange one server with one adapter named `"servo"`; act script `host.browser.engine` then `host.shutdown`; assert the reply names `"servo"` and carries a single engine identity | unrun; fails at HEAD (host never serves), expected pass after the `mod.rs` edit |
| ENG-002 | `config_engine_field_matches_the_engine_the_host_selects` | `tests/acceptance/at009_engine_compatibility.rs` | arrange `Config::default()` (`config.rs:49` -> `"blink"`) and the adapter `main.rs:28` builds; act `host.browser.engine`; assert the reported engine equals `config.browser.engine` | FAILS: config says `blink`, host serves `mock`, nothing reads the field |
| ENG-003 | `capabilities_reply_reports_every_declared_capability` | `tests/acceptance/at008_engine_lifecycle.rs` | arrange the mock adapter; act `host.browser.capabilities`; assert all four `BackendCapabilities` fields are present | FAILS: `browser.rs:49-53` drops `supports_permissions` (`ports/engine.rs:37`) |
| ENG-004 | `engine_crash_after_startup_keeps_the_terminal_core_serving` | `tests/acceptance/at008_engine_lifecycle.rs` | arrange the mock adapter with `simulate_crash("renderer")` before `run()`; act `host.status`, `host.terminal.create`, `host.shutdown`; assert `run().is_ok()` and both replies exist | unrun; fails at HEAD, expected pass after the edit. The post-startup crash case is untested |
| ENG-005 | `each_enabled_engine_passes_its_declared_integration_suite` | `tests/acceptance/at009_engine_compatibility.rs` | arrange the engines compiled into this build; act the shared suite (`tests/support/engine_conformance.rs`) per engine; assert each passes and the list is non-empty | FAILS: no suite; the only compiled engine is mock, so "enabled" is undefined |
| ENG-006 | `enabled_engines_share_one_navigation_permissions_download_lifecycle_scenario_set` | `tests/acceptance/at009_engine_compatibility.rs` | arrange two compiled engines; act the same four scenarios; assert identical engine-neutral outcomes | FAILS: only mock compiles (blink needs a feature, servo is undeclared) |
| ENG-007 | `engine_neutral_ports_and_api_expose_no_engine_handle_types` | `tests/meta/engine_boundary.rs` | arrange the source of `src/browser/ports/**` and `src/browser/api/**`; act scan for `*mut`, `c_void`, `ChromiumEmbedder`, `ServoEmbedder`, `V8`; assert no hit | PASSES for ports/api (`ports/engine.rs` is `SessionId` + plain data); the two `pub` embedder structs are reachable in `adapters/` (`blink.rs:28`, `servo.rs:25`) |
| ENG-008 | `unavailable_engine_is_reported_without_silent_fallback` | `tests/acceptance/at008_engine_lifecycle.rs:111` | already written: assert `engine == "servo"`, `state == "unavailable"`, non-empty `reason` | unrun; fails at HEAD, expected pass after the edit |
| ENG-009 | `experimental_engine_is_marked_in_engine_and_capability_replies` | `tests/acceptance/at008_engine_lifecycle.rs` | arrange the mock adapter (`mock.rs:72`); act both browser routes; assert `experimental` and `is_experimental` are true | PASSES today (`browser.rs:35,52`); the servo case must be added |
| ENG-010 | `engine_removal_leaves_the_user_profile_untouched` | `tests/acceptance/at009_engine_compatibility.rs` | arrange a temp workspace with a profile marker file; act run the host with the engine unavailable, open a session, shut down; assert the marker is byte-identical | FAILS: no browser profile store exists — `BrowserConfig.profile` is a `String` (`ports/engine.rs:20`) that the mock drops (`mock.rs:85-98` uses only `config.url`); it round-trips only through the config file (`file_storage.rs:64,91`), which no host path loads (`ConfigService` has no caller) |
| AT-008 | `unavailable_browser_engine_does_not_stop_the_terminal_core` and `unavailable_engine_is_reported_without_silent_fallback` | `tests/acceptance/at008_engine_lifecycle.rs:92,111` | registered at `Cargo.toml:58-60`; drive the host through `ScriptedTransport` (`tests/support/mod.rs:24`) | unrun; fails at HEAD; the file is untracked and no log covers it |
| AT-009 | `each_enabled_engine_passes_its_declared_integration_suite` | `tests/acceptance/at009_engine_compatibility.rs` (new file plus a new `[[test]]` in `Cargo.toml`) | as ENG-005/ENG-006 | FAILS: the file does not exist; CI-004 is unimplemented |

## 5 Findings, ranked, and the tests to write first

1. **Fatal engine at HEAD** — `server.rs:65` `?` stops the host before it serves.
   Fixed in the working tree, never executed. This is AT-008's whole subject.
2. **`--features servo` is a no-op** — `adapters/mod.rs` never declares `servo`,
   so `servo.rs` (300 lines, 5 tests) is dead source and a build that "enables
   servo" silently ships mock. This is the ENG-008 silent-substitution risk in
   its most concrete form.
3. **No selection mechanism** — `Config.browser.engine` is read by nobody
   (`config.rs:23,49`); the shipped engine is `mock`. ENG-002 and §15.1 ("a
   configuration must fail at build time if required components are missing")
   are unmet.
4. **Crash after startup is invisible** — no IPC route for `last_crash_reason()`
   (`ports/engine.rs:70`); OBS-003 is half met and AT-008's crash half untested.
5. **`supports_permissions` is declared and never reported** (ENG-003).
6. **Adjacent, out of scope:** the capability policy is dead on the IPC path —
   `_policy` is unused (`mod.rs:22,47,103`), so `host.browser.open` and every
   other route skip `validate_request` (`api/host_api.rs:81`). Relevant to AT-005.
7. **Line ceiling:** `servo.rs` 300, `blink_install.rs` 428, `blink.rs` 294,
   `host_api.rs` 203 — all over 200; `servo.rs` needs the FFI block split out
   before it is wired up.

Write first, in this order: (a) run at008's two existing tests one process at a
time via `scripts/run-tests-individually.sh`, because AT-008 currently has no
evidence either way; (b) `capabilities_reply_reports_every_declared_capability`
— smallest failing test, pins ENG-003; (c) `config_engine_field_matches_the_
engine_the_host_selects` — pins ENG-002 and forces the selection seam; (d)
`engine_crash_after_startup_keeps_the_terminal_core_serving`; (e) the shared
suite, only once a second engine actually compiles.
