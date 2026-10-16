//! `stackd`: a counter over HTTP. `GET /health` answers ok, `GET /hit` increments a
//! counter that survives restarts when `DATA_FILE` lives on a volume.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

/// Routes one request line to a status code and body.
pub fn route(line: &str, count: &mut u64) -> (u16, String) {
    match line.trim() {
        l if l.starts_with("GET /health ") => (200, "ok".to_string()),
        l if l.starts_with("GET /hit ") => {
            *count += 1;
            (200, count.to_string())
        }
        _ => (404, "not found".to_string()),
    }
}

/// Renders a minimal HTTP/1.0 response.
pub fn response(status: u16, body: &str) -> String {
    let reason = if status == 200 { "OK" } else { "Not Found" };
    format!(
        "HTTP/1.0 {status} {reason}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// Reads the saved counter; a missing or unreadable file counts as zero.
pub fn load_count(path: &Path) -> u64 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// Saves the counter.
pub fn save_count(path: &Path, n: u64) -> std::io::Result<()> {
    std::fs::write(path, n.to_string())
}

/// Serves one connection: read the request line, write the response.
pub fn handle<S: Read + Write>(stream: S, count: &mut u64) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let (status, body) = route(&line, count);
    reader
        .get_mut()
        .write_all(response(status, &body).as_bytes())
}

/// Accepts connections until `limit` have been served (None = forever), saving the counter after each.
pub fn serve(listener: &TcpListener, data: &Path, limit: Option<usize>) -> std::io::Result<()> {
    let mut count = load_count(data);
    let mut served = 0;
    for stream in listener.incoming() {
        handle(stream?, &mut count)?;
        save_count(data, count)?;
        served += 1;
        if Some(served) == limit {
            break;
        }
    }
    Ok(())
}

/// Health probe for container healthchecks: true when `/health` answers 200.
pub fn probe(addr: &str) -> bool {
    let Ok(mut s) = TcpStream::connect(addr) else {
        return false;
    };
    let mut out = String::new();
    s.write_all(b"GET /health HTTP/1.0\r\n\r\n").is_ok()
        && s.read_to_string(&mut out).is_ok()
        && out.starts_with("HTTP/1.0 200")
}
