//! Writing one HTTP/1.1 response.
//!
//! Every response closes the connection, so a browser reconnects per request.
//! That is slower than keep-alive and far simpler to reason about, which is the
//! right trade for a shell served to one local engine.

use std::io::Write;
use std::net::TcpStream;

pub struct Response {
    pub status: u16,
    pub reason: &'static str,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

impl Response {
    pub fn html(body: String) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type: "text/html; charset=utf-8",
            body: body.into_bytes(),
        }
    }

    pub fn json(body: String) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type: "application/json",
            body: body.into_bytes(),
        }
    }

    pub fn asset(content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type,
            body,
        }
    }

    pub fn forbidden() -> Self {
        Self::text(
            403,
            "Forbidden",
            "the host API needs the token from the served page\n",
        )
    }

    pub fn not_found() -> Self {
        Self::text(404, "Not Found", "not found\n")
    }

    pub fn bad_request(message: &str) -> Self {
        Self::text(400, "Bad Request", &format!("{}\n", message))
    }

    pub fn text(status: u16, reason: &'static str, body: &str) -> Self {
        Self {
            status,
            reason,
            content_type: "text/plain; charset=utf-8",
            body: body.as_bytes().to_vec(),
        }
    }

    pub fn write_to(&self, stream: &mut TcpStream) -> Result<(), String> {
        let head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
            self.status, self.reason, self.content_type, self.body.len()
        );
        stream
            .write_all(head.as_bytes())
            .map_err(|e| e.to_string())?;
        stream.write_all(&self.body).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())
    }
}
