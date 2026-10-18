//! The loopback HTTP transport for the desktop shell (SRD UI-001, UI-008).
//!
//! A browser engine cannot speak stdio, so the shell is served over HTTP on
//! loopback and calls the same typed, versioned host API through `POST /api`.
//! Any engine that renders HTML and runs JavaScript can host the shell, which
//! is what makes the engine interchangeable (SRD §5.2).
//!
//! Hazard: the port is reachable by anything on this machine that finds it.
//! The token minted at startup, injected into the served index.html and
//! required on every API call, is what stops a random local page from driving
//! the host (SRD SEC-014, UI-009).

use super::assets;
use super::request::{self, Request};
use super::response::Response;
use crate::ipc::server::Core;
use std::fs;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

/// The marker in `index.html` that the token replaces.
const TOKEN_MARKER: &str = "__OKO_HOST_TOKEN__";
const TOKEN_HEADER: &str = "x-oko-token";

pub struct HttpUiServer {
    core: Core,
    ui_root: PathBuf,
    token: String,
    listener: TcpListener,
}

impl HttpUiServer {
    /// Bind on loopback. Port 0 asks the OS for a free port, which is what a
    /// test wants and what the host prints for the engine to open.
    pub fn bind(core: Core, ui_root: &Path, port: u16) -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .map_err(|e| format!("bind 127.0.0.1:{}: {}", port, e))?;
        Ok(Self {
            core,
            ui_root: ui_root.to_path_buf(),
            token: mint_token()?,
            listener,
        })
    }

    pub fn address(&self) -> String {
        self.listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "127.0.0.1:0".into())
    }

    pub fn url(&self) -> String {
        format!("http://{}/", self.address())
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// Serve until the process is asked to stop.
    pub fn serve(mut self) -> Result<(), String> {
        self.core.start();
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if let Err(e) = self.handle(stream) {
                        eprintln!("oko-ui: {}", e);
                    }
                }
                Err(e) => return Err(format!("accept: {}", e)),
            }
        }
    }

    /// One request, for tests: same code path as `serve`.
    pub fn serve_once(&mut self) -> Result<(), String> {
        let (stream, _) = self.listener.accept().map_err(|e| e.to_string())?;
        self.handle(stream)
    }

    fn handle(&mut self, mut stream: TcpStream) -> Result<(), String> {
        let request = match request::read(&stream) {
            Ok(request) => request,
            Err(e) => return Response::bad_request(&e).write_to(&mut stream),
        };
        let response = match (request.method.as_str(), request.path.as_str()) {
            ("GET", path) => self.get(path),
            ("POST", "/api") => self.api(&request),
            ("POST", _) => Response::not_found(),
            _ => Response::text(405, "Method Not Allowed", "GET or POST\n"),
        };
        response.write_to(&mut stream)
    }

    fn get(&self, path: &str) -> Response {
        let Some(file) = assets::resolve(&self.ui_root, path) else {
            return Response::not_found();
        };
        let Ok(bytes) = fs::read(&file) else {
            return Response::not_found();
        };
        let content_type = assets::content_type(&file);
        if file.ends_with("index.html") {
            let page = String::from_utf8_lossy(&bytes).replace(TOKEN_MARKER, &self.token);
            return Response::html(page);
        }
        Response::asset(content_type, bytes)
    }

    /// `POST /api` carries one typed, versioned IPC message.
    fn api(&mut self, request: &Request) -> Response {
        match request.header(TOKEN_HEADER) {
            Some(token) if token == self.token => {}
            _ => return Response::forbidden(),
        }
        let line = String::from_utf8_lossy(&request.body);
        let message = match crate::ipc::message::Message::from_line(line.trim()) {
            Ok(message) => message,
            Err(e) => return Response::bad_request(&format!("bad message: {}", e)),
        };
        Response::json(self.core.handle(&message).to_line())
    }
}

/// 128 bits from the kernel's random source. A token that is not random is not
/// a token, so a failure here stops the host rather than weakening the check.
fn mint_token() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|e| format!("read /dev/urandom: {}", e))?;
    Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
}
