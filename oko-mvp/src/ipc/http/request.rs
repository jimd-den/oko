//! Reading one HTTP/1.1 request off a TCP stream.
//!
//! Deliberately small: this serves a local desktop shell, not the Internet.
//! No keep-alive, no chunked bodies, no pipelining. Anything it does not
//! understand is a 400 rather than a guess.
//!
//! Hazard: the body length comes from the request, so it is bounded before a
//! single byte is allocated (SRD SEC-006).

use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;

/// The largest request body the host will read: a command, not an upload.
const MAX_BODY: usize = 1 << 20;

pub struct Request {
    pub method: String,
    pub path: String,
    headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

pub fn read(stream: &TcpStream) -> Result<Request, String> {
    let mut reader = BufReader::new(stream);

    let mut request_line = String::new();
    if reader
        .read_line(&mut request_line)
        .map_err(|e| e.to_string())?
        == 0
    {
        return Err("empty request".into());
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("no method")?.to_string();
    let path = parts.next().ok_or("no path")?.to_string();

    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }

    let length: usize = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse().ok())
        .unwrap_or(0);
    if length > MAX_BODY {
        return Err(format!("body too large: {} bytes", length));
    }

    let mut body = vec![0u8; length];
    if length > 0 {
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    }
    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}
