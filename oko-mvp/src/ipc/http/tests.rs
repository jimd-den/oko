//! The loopback transport, driven the way an engine drives it.
//!
//! Each test issues a real HTTP request over a real socket, because the whole
//! point of this transport is that something outside this process can use it.

use super::*;
use crate::adapters::{linux_process::LinuxProcessAdapter, linux_pty::LinuxPtyAdapter};
use crate::application::session_service::SessionService;
use crate::browser::adapters::{blink_install::BlinkInstaller, mock::MockBrowserEngineAdapter};
use crate::domain::capability::{Capability, CapabilityPolicy};
use crate::ipc::json::{self, object};
use crate::ipc::message::Message;
use crate::ipc::server::Core;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

fn server() -> HttpUiServer {
    let core = Core::new(
        Box::new(LinuxProcessAdapter::new()),
        Box::new(LinuxPtyAdapter::new()),
        SessionService::new(),
        CapabilityPolicy::new()
            .with(Capability::TerminalCreate)
            .with(Capability::BrowserNavigate),
        Box::new(MockBrowserEngineAdapter::new()),
        BlinkInstaller::new(&std::env::temp_dir().join("oko-http-test")),
    );
    let ui_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui");
    HttpUiServer::bind(core, &ui_root, 0).expect("bind loopback")
}

/// Send a raw request on a fresh connection and read the whole reply.
fn round_trip(server: &mut HttpUiServer, request: String) -> String {
    let address = server.address();
    let client = std::thread::spawn(move || {
        let mut stream = TcpStream::connect(address).expect("connect");
        stream.write_all(request.as_bytes()).expect("write");
        let mut reply = String::new();
        stream.read_to_string(&mut reply).expect("read");
        reply
    });
    server.serve_once().expect("serve one request");
    client.join().expect("client thread")
}

fn get(server: &mut HttpUiServer, path: &str) -> String {
    round_trip(
        server,
        format!("GET {} HTTP/1.1\r\nHost: localhost\r\n\r\n", path),
    )
}

fn post_api(server: &mut HttpUiServer, token: Option<&str>, body: &str) -> String {
    let auth = match token {
        Some(token) => format!("X-Oko-Token: {}\r\n", token),
        None => String::new(),
    };
    round_trip(
        server,
        format!(
            "POST /api HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n{}Content-Length: {}\r\n\r\n{}",
            auth,
            body.len(),
            body
        ),
    )
}

#[test]
fn the_desktop_page_is_served_with_the_session_token() {
    let mut server = server();
    let token = server.token().to_string();

    let reply = get(&mut server, "/");

    assert!(
        reply.starts_with("HTTP/1.1 200 OK"),
        "reply was {:?}",
        &reply[..40.min(reply.len())]
    );
    assert!(reply.contains("text/html"));
    assert!(reply.contains(&token), "the page must carry the token");
    assert!(
        !reply.contains("__OKO_HOST_TOKEN__"),
        "the marker must be replaced"
    );
}

#[test]
fn an_asset_is_served_with_its_content_type() {
    let mut server = server();
    let reply = get(&mut server, "/styles/main.css");
    assert!(reply.starts_with("HTTP/1.1 200 OK"));
    assert!(
        reply.contains("text/css"),
        "reply head was {:?}",
        &reply[..60.min(reply.len())]
    );
}

#[test]
fn a_path_outside_the_ui_directory_is_not_served() {
    let mut server = server();
    assert!(get(&mut server, "/../Cargo.toml").starts_with("HTTP/1.1 404"));
    assert!(get(&mut server, "/nope.js").starts_with("HTTP/1.1 404"));
}

#[test]
fn the_host_api_refuses_a_call_without_the_token() {
    let mut server = server();
    let message = Message::request("1", "host.status", object(vec![]));
    let reply = post_api(&mut server, None, &message.to_line());
    assert!(
        reply.starts_with("HTTP/1.1 403"),
        "reply was {:?}",
        &reply[..40.min(reply.len())]
    );
}

#[test]
fn the_host_api_answers_status_with_the_token() {
    let mut server = server();
    let token = server.token().to_string();
    let message = Message::request("1", "host.status", object(vec![]));

    let reply = post_api(&mut server, Some(&token), &message.to_line());

    assert!(reply.starts_with("HTTP/1.1 200 OK"));
    let body = reply.split("\r\n\r\n").nth(1).unwrap_or("");
    let parsed = Message::from_line(body.trim()).expect("the reply must be a valid message");
    assert_eq!(parsed.route, "host.status.reply");
    assert_eq!(
        json::get(&parsed.payload, "core").and_then(json::as_str),
        Some("running")
    );
}

#[test]
fn the_host_api_rejects_a_malformed_message() {
    let mut server = server();
    let token = server.token().to_string();
    let reply = post_api(&mut server, Some(&token), "not a message");
    assert!(reply.starts_with("HTTP/1.1 400"));
}
