use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;

use crate::error::{CodexLagError, Result};

pub struct LoopbackCallbackPayload {
    pub code: String,
    pub state: String,
}

pub fn read_loopback_callback(
    stream: &mut TcpStream,
    expected_path: &str,
) -> Result<LoopbackCallbackPayload> {
    let mut buffer = [0_u8; 8192];
    let read = stream.read(&mut buffer).map_err(|error| {
        CodexLagError::new(format!("failed to read openai loopback callback request: {error}"))
    })?;
    if read == 0 {
        return Err(CodexLagError::new(
            "openai loopback callback request was empty",
        ));
    }

    let request = String::from_utf8_lossy(&buffer[..read]);
    let request_line = request
        .lines()
        .next()
        .ok_or_else(|| CodexLagError::new("openai loopback callback request was malformed"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    if method != "GET" {
        return Err(CodexLagError::new(format!(
            "openai loopback callback expected GET but received {method}"
        )));
    }
    let (path, query) = target.split_once('?').ok_or_else(|| {
        CodexLagError::new("openai loopback callback request did not include query parameters")
    })?;
    if path != expected_path {
        return Err(CodexLagError::new(format!(
            "openai loopback callback expected path '{expected_path}' but received '{path}'"
        )));
    }

    let params = parse_query(query);
    let code = params
        .get("code")
        .cloned()
        .ok_or_else(|| CodexLagError::new("openai loopback callback did not include a code"))?;
    let state = params
        .get("state")
        .cloned()
        .ok_or_else(|| CodexLagError::new("openai loopback callback did not include a state"))?;
    Ok(LoopbackCallbackPayload { code, state })
}

pub fn respond_loopback_success(stream: &mut TcpStream) -> Result<()> {
    write_response(
        stream,
        "200 OK",
        "<html><body><h1>OpenAI login complete</h1><p>You can return to CodexLAG.</p></body></html>",
    )
}

pub fn respond_loopback_error(stream: &mut TcpStream, message: &str) -> Result<()> {
    write_response(
        stream,
        "400 Bad Request",
        &format!(
            "<html><body><h1>OpenAI login failed</h1><p>{}</p></body></html>",
            html_escape(message)
        ),
    )
}

fn write_response(stream: &mut TcpStream, status: &str, body: &str) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).map_err(|error| {
        CodexLagError::new(format!("failed to write openai loopback callback response: {error}"))
    })
}

fn parse_query(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        params.insert(percent_decode(key), percent_decode(value));
    }
    params
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let value = &input[index + 1..index + 3];
                if let Ok(byte) = u8::from_str_radix(value, 16) {
                    decoded.push(byte);
                    index += 3;
                } else {
                    decoded.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn html_escape(message: &str) -> String {
    message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
