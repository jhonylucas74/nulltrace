//! Minimal HTTP/1.0-style protocol for VM-to-VM communication.
//! Text-based request-response format for use over the game's net layer.

#![allow(dead_code)]

use std::fmt;

/// HTTP method for requests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "PATCH" => Some(HttpMethod::Patch),
            "DELETE" => Some(HttpMethod::Delete),
            "HEAD" => Some(HttpMethod::Head),
            _ => None,
        }
    }
}

/// HTTP request (client → server).
#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn get(path: &str) -> Self {
        Self {
            method: HttpMethod::Get,
            path: path.to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn post(path: &str, body: &[u8]) -> Self {
        Self {
            method: HttpMethod::Post,
            path: path.to_string(),
            headers: Vec::new(),
            body: body.to_vec(),
        }
    }

    pub fn put(path: &str, body: &[u8]) -> Self {
        Self {
            method: HttpMethod::Put,
            path: path.to_string(),
            headers: Vec::new(),
            body: body.to_vec(),
        }
    }

    pub fn patch(path: &str, body: &[u8]) -> Self {
        Self {
            method: HttpMethod::Patch,
            path: path.to_string(),
            headers: Vec::new(),
            body: body.to_vec(),
        }
    }

    pub fn delete(path: &str) -> Self {
        Self {
            method: HttpMethod::Delete,
            path: path.to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn head(path: &str) -> Self {
        Self {
            method: HttpMethod::Head,
            path: path.to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(
            format!("{} {} HTTP/1.0\r\n", self.method.as_str(), self.path).as_bytes(),
        );
        for (k, v) in &self.headers {
            out.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
        }
        if !self.body.is_empty() {
            out.extend_from_slice(format!("Content-Length: {}\r\n", self.body.len()).as_bytes());
        }
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&self.body);
        out
    }
}

/// HTTP response (server → client).
#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub reason_phrase: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn ok(body: &[u8]) -> Self {
        Self {
            status_code: 200,
            reason_phrase: "OK".to_string(),
            headers: Vec::new(),
            body: body.to_vec(),
        }
    }

    pub fn not_found() -> Self {
        Self {
            status_code: 404,
            reason_phrase: "Not Found".to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(
            format!("HTTP/1.0 {} {}\r\n", self.status_code, self.reason_phrase).as_bytes(),
        );
        for (k, v) in &self.headers {
            out.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
        }
        if !self.body.is_empty() {
            out.extend_from_slice(format!("Content-Length: {}\r\n", self.body.len()).as_bytes());
        }
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&self.body);
        out
    }
}

/// Parse error for HTTP protocol.
#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parse an HTTP request from raw bytes.
pub fn parse_http_request(data: &[u8]) -> Result<HttpRequest, ParseError> {
    let s = std::str::from_utf8(data).map_err(|e| ParseError(e.to_string()))?;
    let mut parts = s.splitn(2, "\r\n\r\n");
    let head = parts.next().ok_or_else(|| ParseError("empty request".to_string()))?;
    let body_str = parts.next().unwrap_or("");
    let body = body_str.as_bytes().to_vec();

    let mut lines = head.lines();
    let first = lines
        .next()
        .ok_or_else(|| ParseError("missing request line".to_string()))?;
    let req_parts: Vec<&str> = first.split_whitespace().collect();
    if req_parts.len() < 2 {
        return Err(ParseError("invalid request line".to_string()));
    }
    let method = HttpMethod::parse(req_parts[0])
        .ok_or_else(|| ParseError(format!("unknown method: {}", req_parts[0])))?;
    let path = req_parts[1].to_string();

    let mut headers = Vec::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.push((
                name.trim().to_string(),
                value.trim().to_string(),
            ));
        }
    }

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

/// Parse an HTTP response from raw bytes.
pub fn parse_http_response(data: &[u8]) -> Result<HttpResponse, ParseError> {
    let s = std::str::from_utf8(data).map_err(|e| ParseError(e.to_string()))?;
    let mut parts = s.splitn(2, "\r\n\r\n");
    let head = parts.next().ok_or_else(|| ParseError("empty response".to_string()))?;
    let body_str = parts.next().unwrap_or("");
    let body = body_str.as_bytes().to_vec();

    let mut lines = head.lines();
    let first = lines
        .next()
        .ok_or_else(|| ParseError("missing status line".to_string()))?;
    let status_parts: Vec<&str> = first.splitn(3, ' ').collect();
    if status_parts.len() < 2 {
        return Err(ParseError("invalid status line".to_string()));
    }
    let status_code = status_parts[1]
        .parse::<u16>()
        .map_err(|_| ParseError("invalid status code".to_string()))?;
    let reason_phrase = status_parts[2..].join(" ");

    let mut headers = Vec::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.push((
                name.trim().to_string(),
                value.trim().to_string(),
            ));
        }
    }

    Ok(HttpResponse {
        status_code,
        reason_phrase,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_get_to_bytes() {
        let req = HttpRequest::get("/");
        let bytes = req.to_bytes();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.starts_with("GET / HTTP/1.0\r\n"));
    }

    #[test]
    fn test_request_post_to_bytes() {
        let req = HttpRequest::post("/api", b"hello");
        let bytes = req.to_bytes();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("POST /api HTTP/1.0"));
        assert!(s.contains("Content-Length: 5"));
        assert!(s.ends_with("hello"));
    }

    #[test]
    fn test_parse_request() {
        let raw = b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n";
        let req = parse_http_request(raw).unwrap();
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.path, "/");
        assert!(req.body.is_empty());
    }

    #[test]
    fn test_response_ok_to_bytes() {
        let res = HttpResponse::ok(b"Hello");
        let bytes = res.to_bytes();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.starts_with("HTTP/1.0 200 OK\r\n"));
        assert!(s.contains("Content-Length: 5"));
    }

    #[test]
    fn test_parse_response() {
        let raw = b"HTTP/1.0 200 OK\r\nContent-Type: text/plain\r\n\r\nHello";
        let res = parse_http_response(raw).unwrap();
        assert_eq!(res.status_code, 200);
        assert_eq!(res.reason_phrase, "OK");
        assert_eq!(res.body, b"Hello");
    }
}
