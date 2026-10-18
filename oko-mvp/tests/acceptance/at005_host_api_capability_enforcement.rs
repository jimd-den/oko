//! Acceptance test AT-005: host API capability enforcement.
//!
//! Pass condition: an unauthorized call is refused by the native host, not by
//! the caller's good manners. A capability declared in a UI document grants
//! nothing (SRD SEC-002, §8.3, §10.2).
//!
//! Every test here drives the host through the real dispatch path, because the
//! enforcement lives there and not in a handler.

#[path = "../support/mod.rs"]
mod support;

use oko_core::browser::adapters::mock::MockBrowserEngineAdapter;
use oko_core::domain::capability::{Capability, CapabilityPolicy};
use oko_core::ipc::json::{self, object};
use oko_core::ipc::message::Message;
use support::fake_engines::run_host_with_policy;

fn empty_policy() -> CapabilityPolicy {
    CapabilityPolicy::new()
}

fn request(id: &str, route: &str) -> Message {
    Message::request(id, route, object(vec![]))
}

#[test]
fn a_route_needing_no_capability_still_answers_without_one() {
    let script = vec![
        request("1", "host.status"),
        request("2", "host.shutdown"),
    ];

    let (transport, run) = run_host_with_policy(
        script,
        Box::new(MockBrowserEngineAdapter::new()),
        empty_policy(),
    );

    assert!(run.is_ok(), "host exited with {:?}", run);
    assert!(
        transport.reply_for("host.status").is_some(),
        "status needs no capability and must not be blocked"
    );
}

#[test]
fn terminal_create_is_refused_when_the_policy_does_not_grant_it() {
    let script = vec![
        request("1", "host.terminal.create"),
        request("2", "host.shutdown"),
    ];

    let (transport, run) = run_host_with_policy(
        script,
        Box::new(MockBrowserEngineAdapter::new()),
        empty_policy(),
    );

    assert!(run.is_ok(), "host exited with {:?}", run);
    let error = transport
        .error_for("host.terminal.create")
        .expect("an unpermitted call must be refused by the host");
    assert!(error.contains("permission denied"), "error was {:?}", error);
}

#[test]
fn browser_open_is_refused_when_the_policy_does_not_grant_it() {
    let script = vec![
        Message::request(
            "1",
            "host.browser.open",
            object(vec![("url".into(), json::str("https://example.com"))]),
        ),
        request("2", "host.shutdown"),
    ];

    let (transport, run) = run_host_with_policy(
        script,
        Box::new(MockBrowserEngineAdapter::new()),
        CapabilityPolicy::new().with(Capability::TerminalCreate),
    );

    assert!(run.is_ok(), "host exited with {:?}", run);
    let error = transport
        .error_for("host.browser.open")
        .expect("browser.navigate was not granted");
    assert!(error.contains("permission denied"), "error was {:?}", error);
}
