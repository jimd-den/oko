//! Acceptance test AT-008: browser engine lifecycle.
//!
//! Pass condition: a browser engine that is unavailable or crashed must not
//! take down the terminal core, and the failure must be reported clearly
//! rather than silently replaced by another engine
//! (SRD ENG-004, ENG-008, and §4.2 "a failing optional service cannot prevent
//! the core terminal from starting").
//!
//! The host is driven through an in-memory IPC transport, so the whole message
//! loop runs without a terminal attached.

#[path = "../support/mod.rs"]
mod support;

use oko_core::ipc::json::{self, object};
use oko_core::ipc::message::Message;
use support::fake_engines::{run_host, CrashingEngine, UnavailableEngine};

#[test]
fn unavailable_browser_engine_does_not_stop_the_terminal_core() {
    let script = vec![
        Message::request("1", "host.status", object(vec![])),
        Message::request("2", "host.terminal.create", object(vec![])),
        Message::request("3", "host.shutdown", object(vec![])),
    ];

    let (transport, run) = run_host(script, Box::new(UnavailableEngine { name: "servo" }));

    assert!(run.is_ok(), "host exited with {:?} instead of serving", run);
    let status = transport.reply_for("host.status").expect("host.status must be answered");
    assert_eq!(json::get(&status, "core").and_then(json::as_str), Some("running"));
    let created = transport
        .reply_for("host.terminal.create")
        .expect("terminal must still be usable without a browser engine");
    assert!(json::get(&created, "session_id").is_some(), "reply was {:?}", created);
}

#[test]
fn unavailable_engine_is_reported_without_silent_fallback() {
    let script = vec![
        Message::request("1", "host.browser.engine", object(vec![])),
        Message::request("2", "host.shutdown", object(vec![])),
    ];

    let (transport, run) = run_host(script, Box::new(UnavailableEngine { name: "servo" }));

    assert!(run.is_ok(), "host exited with {:?} instead of serving", run);
    let reply = transport.reply_for("host.browser.engine").expect("engine must be reported");
    assert_eq!(
        json::get(&reply, "engine").and_then(json::as_str),
        Some("servo"),
        "the requested engine must not be silently replaced (ENG-008)"
    );
    assert_eq!(
        json::get(&reply, "state").and_then(json::as_str),
        Some("unavailable"),
        "reply was {:?}",
        reply
    );
    let reason = json::get(&reply, "reason").and_then(json::as_str).unwrap_or("");
    assert!(!reason.is_empty(), "an unavailable engine must say why (OBS-003)");
}

#[test]
fn engine_crash_after_startup_keeps_the_terminal_core_serving() {
    let script = vec![
        Message::request(
            "1",
            "host.browser.open",
            object(vec![("url".into(), json::str("https://example.com"))]),
        ),
        Message::request("2", "host.browser.engine", object(vec![])),
        Message::request("3", "host.status", object(vec![])),
        Message::request("4", "host.terminal.create", object(vec![])),
        Message::request("5", "host.shutdown", object(vec![])),
    ];

    let (transport, run) = run_host(script, Box::new(CrashingEngine { name: "mock" }));

    assert!(run.is_ok(), "host exited with {:?} instead of serving", run);
    let error = transport
        .error_for("host.browser.open")
        .expect("a crashed engine must fail the request that hit it");
    assert!(error.contains("Crash"), "error was {:?}", error);
    assert!(
        transport.reply_for("host.status").is_some(),
        "the core must keep answering after an engine crash (ENG-004)"
    );
    assert!(
        transport.reply_for("host.terminal.create").is_some(),
        "the terminal must survive an engine crash (ENG-004)"
    );
    let engine = transport.reply_for("host.browser.engine").expect("engine reply");
    assert_eq!(
        json::get(&engine, "last_crash").and_then(json::as_str),
        Some("renderer process terminated"),
        "the crash reason must be reported (OBS-003), reply was {:?}",
        engine
    );
}
