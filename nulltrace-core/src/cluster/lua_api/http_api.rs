//! Lua API for HTTP-like protocol (build/parse requests and responses over net layer).

#![allow(dead_code)]

use crate::net::http_proto::{parse_http_request, parse_http_response, HttpMethod, HttpRequest, HttpResponse};
use mlua::{Lua, Result, Value};

/// Register the `http` table on the Lua state.
pub fn register(lua: &Lua) -> Result<()> {
    let http = lua.create_table()?;

    // http.build_request(method, path, body?) -> string (raw bytes)
    http.set(
        "build_request",
        lua.create_function(|lua, (method_str, path, body): (String, String, Option<String>)| {
            let method = HttpMethod::parse(&method_str)
                .ok_or_else(|| mlua::Error::runtime(format!("Unknown HTTP method: {}", method_str)))?;
            let body_bytes = body.as_deref().map(|s| s.as_bytes()).unwrap_or(&[]);
            let req = match method {
                HttpMethod::Get => HttpRequest::get(&path),
                HttpMethod::Post => HttpRequest::post(&path, body_bytes),
                HttpMethod::Put => HttpRequest::put(&path, body_bytes),
                HttpMethod::Patch => HttpRequest::patch(&path, body_bytes),
                HttpMethod::Delete => HttpRequest::delete(&path),
                HttpMethod::Head => HttpRequest::head(&path),
            };
            Ok(Value::String(lua.create_string(&req.to_bytes())?))
        })?,
    )?;

    // http.parse_request(data) -> { method, path, headers, body } | nil on error
    http.set(
        "parse_request",
        lua.create_function(|lua, data: String| {
            let req = parse_http_request(data.as_bytes())
                .map_err(|e| mlua::Error::runtime(e.0))?;
            let t = lua.create_table()?;
            t.set("method", req.method.as_str())?;
            t.set("path", req.path)?;
            let headers = lua.create_table()?;
            for (i, (k, v)) in req.headers.iter().enumerate() {
                let row = lua.create_table()?;
                row.set(1, k.as_str())?;
                row.set(2, v.as_str())?;
                headers.set(i + 1, row)?;
            }
            t.set("headers", headers)?;
            t.set("body", String::from_utf8_lossy(&req.body).to_string())?;
            Ok(Value::Table(t))
        })?,
    )?;

    // http.build_response(status, body?, headers?) -> string (raw bytes)
    // headers: optional table { ["Content-Type"] = "application/x-ntml", ... }
    http.set(
        "build_response",
        lua.create_function(|lua, (status, body, headers): (u16, Option<String>, Option<mlua::Table>)| {
            let body_bytes = body.as_deref().map(|s| s.as_bytes()).unwrap_or(&[]);
            let mut res = match status {
                200 => HttpResponse::ok(body_bytes),
                404 => HttpResponse {
                    status_code: 404,
                    reason_phrase: "Not Found".to_string(),
                    headers: Vec::new(),
                    body: body_bytes.to_vec(),
                },
                _ => HttpResponse {
                    status_code: status,
                    reason_phrase: "Unknown".to_string(),
                    headers: Vec::new(),
                    body: body_bytes.to_vec(),
                },
            };
            if let Some(t) = headers {
                for pair in t.pairs::<String, String>() {
                    if let Ok((k, v)) = pair {
                        res = res.with_header(&k, &v);
                    }
                }
            }
            Ok(Value::String(lua.create_string(&res.to_bytes())?))
        })?,
    )?;

    // http.parse_response(data) -> { status, reason, headers, body } | nil on error
    http.set(
        "parse_response",
        lua.create_function(|lua, data: String| {
            let res = parse_http_response(data.as_bytes())
                .map_err(|e| mlua::Error::runtime(e.0))?;
            let t = lua.create_table()?;
            t.set("status", res.status_code)?;
            t.set("reason", res.reason_phrase)?;
            let headers = lua.create_table()?;
            for (i, (k, v)) in res.headers.iter().enumerate() {
                let row = lua.create_table()?;
                row.set(1, k.as_str())?;
                row.set(2, v.as_str())?;
                headers.set(i + 1, row)?;
            }
            t.set("headers", headers)?;
            t.set("body", String::from_utf8_lossy(&res.body).to_string())?;
            Ok(Value::Table(t))
        })?,
    )?;

    lua.globals().set("http", http)?;
    Ok(())
}
