//! Acceptance test AT-009: engine identity, capabilities and selection.
//!
//! Pass condition: the engine the build and the profile name is the engine that
//! serves, it declares what it can and cannot do, and every enabled backend
//! passes the same shared scenario set
//! (SRD ENG-001, ENG-002, ENG-003, ENG-005, ENG-006, ENG-009, AT-009).

#[path = "../support/mod.rs"]
mod support;

use oko_core::browser::adapters::mock::MockBrowserEngineAdapter;
use oko_core::browser::ports::engine::BrowserEnginePort;
use oko_core::ipc::json::{self, object};
use oko_core::ipc::message::Message;
use support::engine_conformance::{assert_engine_conformance, EngineExpectation};
use support::fake_engines::run_host;
use support::host_binary::{reply_for, run_host_binary};

const MOCK: EngineExpectation = EngineExpectation {
    name: "mock",
    experimental: true,
    supports_downloads: false,
    supports_permissions: false,
    supports_script: true,
};

#[cfg(feature = "blink")]
const BLINK: EngineExpectation = EngineExpectation {
    name: "blink",
    experimental: false,
    supports_downloads: true,
    supports_permissions: true,
    supports_script: false,
};

#[cfg(feature = "servo")]
const SERVO: EngineExpectation = EngineExpectation {
    name: "servo",
    experimental: true,
    supports_downloads: false,
    supports_permissions: false,
    supports_script: false,
};

#[test]
fn capabilities_reply_reports_every_declared_capability() {
    let script = vec![
        Message::request("1", "host.browser.capabilities", object(vec![])),
        Message::request("2", "host.shutdown", object(vec![])),
    ];

    let (transport, _run) = run_host(script, Box::new(MockBrowserEngineAdapter::new()));

    let caps = transport
        .reply_for("host.browser.capabilities")
        .expect("capabilities reply");
    for field in [
        "supports_js",
        "supports_downloads",
        "supports_permissions",
        "is_experimental",
    ] {
        assert!(
            json::get(&caps, field).and_then(json::as_bool).is_some(),
            "ENG-003: {} must be declared, reply was {:?}",
            field,
            caps
        );
    }
}

#[test]
fn host_reports_the_engine_named_on_the_command_line() {
    let script = vec![
        Message::request("1", "host.browser.engine", object(vec![])),
        Message::request("2", "host.terminal.create", object(vec![])),
        Message::request("3", "host.shutdown", object(vec![])),
    ];

    let messages = run_host_binary(&["--engine", "servo"], &script);

    let reply = reply_for(&messages, "host.browser.engine").expect("engine reply");
    assert_eq!(
        json::get(&reply.payload, "engine").and_then(json::as_str),
        Some("servo"),
        "ENG-002: the named engine must be the one that serves"
    );
    assert!(
        reply_for(&messages, "host.terminal.create").is_some(),
        "an unavailable engine must not stop the terminal"
    );
}

#[test]
fn host_does_not_substitute_another_engine_for_an_unknown_name() {
    let script = vec![
        Message::request("1", "host.browser.engine", object(vec![])),
        Message::request("2", "host.shutdown", object(vec![])),
    ];

    let messages = run_host_binary(&["--engine", "frobnicate"], &script);

    let reply = reply_for(&messages, "host.browser.engine").expect("engine reply");
    assert_eq!(
        json::get(&reply.payload, "engine").and_then(json::as_str),
        Some("frobnicate"),
        "ENG-008: an unknown engine must be reported, not replaced by a working one"
    );
    assert_eq!(
        json::get(&reply.payload, "state").and_then(json::as_str),
        Some("unavailable")
    );
}

#[test]
fn mock_engine_passes_the_shared_conformance_suite() {
    let engine: Box<dyn BrowserEnginePort> = Box::new(MockBrowserEngineAdapter::new());
    assert_engine_conformance(engine.as_ref(), &MOCK);
}

#[cfg(feature = "blink")]
#[test]
fn blink_engine_passes_the_shared_conformance_suite() {
    use oko_core::browser::adapters::blink::BlinkBrowserEngineAdapter;
    let engine = BlinkBrowserEngineAdapter::with_embedder(1 as *mut std::ffi::c_void);
    assert_engine_conformance(&engine, &BLINK);
}

#[cfg(feature = "servo")]
#[test]
fn servo_engine_passes_the_shared_conformance_suite() {
    use oko_core::browser::adapters::servo::ServoBrowserEngineAdapter;
    // Nothing in this crate dereferences the embedder handle: it is a token
    // passed back to the engine's own API. A non-null address is enough to
    // exercise the linked path, and the real handle arrives from the pinned
    // Servo build (SRD §14).
    let engine = ServoBrowserEngineAdapter::with_embedder(1 as *mut std::ffi::c_void);
    assert_engine_conformance(&engine, &SERVO);
}
