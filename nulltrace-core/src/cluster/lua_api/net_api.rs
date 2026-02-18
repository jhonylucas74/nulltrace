#![allow(dead_code)]

use super::context::VmContext;
use crate::net::connection::ConnectionState;
use crate::net::ip::Ipv4Addr;
use crate::net::packet::Packet;
use mlua::{Lua, Result};
use std::collections::VecDeque;

/// Resolve host string to IP. "localhost" and "127.0.0.1" (and any 127.x.x.x) map to loopback.
fn resolve_host(host: &str) -> Option<Ipv4Addr> {
    let s = host.trim();
    if s.eq_ignore_ascii_case("localhost") {
        return Some(Ipv4Addr::new(127, 0, 0, 1));
    }
    if let Some(ip) = Ipv4Addr::parse(s) {
        return Some(ip);
    }
    None
}

/// Register the `net` table on the Lua state.
/// Network operations are in-memory (NIC buffers), no DB calls needed.
pub fn register(lua: &Lua) -> Result<()> {
    let net = lua.create_table()?;

    // net.send(ip, port, data) — queue a TCP packet to the outbound buffer. Accepts "localhost" and 127.x.x.x for loopback.
    net.set(
        "send",
        lua.create_function(|lua, (dst_ip_str, dst_port, data): (String, u16, String)| {
            let dst_ip = resolve_host(&dst_ip_str)
                .ok_or_else(|| mlua::Error::runtime(format!("Invalid IP/host: {}", dst_ip_str)))?;

            let mut ctx = lua
                .app_data_mut::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;

            let src_ip = ctx
                .ip
                .ok_or_else(|| mlua::Error::runtime("VM has no IP address"))?;

            ctx.net_outbound.push(Packet::tcp(
                src_ip,
                0, // ephemeral source port
                dst_ip,
                dst_port,
                data.into_bytes(),
            ));

            Ok(())
        })?,
    )?;

    // net.recv() -> { src_ip, src_port, dst_port, data } | nil
    net.set(
        "recv",
        lua.create_function(|lua, ()| {
            let mut ctx = lua
                .app_data_mut::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;

            match ctx.net_inbound.pop_front() {
                Some(pkt) => {
                    let t = lua.create_table()?;
                    t.set("src_ip", pkt.src_ip.to_string())?;
                    t.set("src_port", pkt.src_port)?;
                    t.set("dst_port", pkt.dst_port)?;
                    t.set("data", pkt.payload_str().unwrap_or("").to_string())?;
                    Ok(mlua::Value::Table(t))
                }
                None => Ok(mlua::Value::Nil),
            }
        })?,
    )?;

    // net.listen(port) — one port per process; second process gets "Address already in use"
    net.set(
        "listen",
        lua.create_function(|lua, port: u16| {
            let mut ctx = lua
                .app_data_mut::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;

            let current_pid = ctx.current_pid;
            if let Some(&owner_pid) = ctx.port_owners.get(&port) {
                if owner_pid != current_pid {
                    return Err(mlua::Error::runtime(format!(
                        "Address already in use (port {})",
                        port
                    )));
                }
                // Same process, idempotent
                return Ok(());
            }
            ctx.pending_listen.push((port, current_pid));
            ctx.port_owners.insert(port, current_pid);
            Ok(())
        })?,
    )?;

    // net.ip() -> string | nil
    net.set(
        "ip",
        lua.create_function(|lua, ()| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            match ctx.ip {
                Some(ip) => Ok(mlua::Value::String(lua.create_string(&ip.to_string())?)),
                None => Ok(mlua::Value::Nil),
            }
        })?,
    )?;

    // net.connect(host, port) -> connection table (conn:send, conn:recv, conn:close). Uses ephemeral port; no net.listen(0) needed. Accepts "localhost" and 127.x.x.x for loopback.
    net.set(
        "connect",
        lua.create_function(|lua, (host_str, remote_port): (String, u16)| {
            let remote_ip = resolve_host(&host_str)
                .ok_or_else(|| mlua::Error::runtime(format!("Invalid IP/host: {}", host_str)))?;

            let mut ctx = lua
                .app_data_mut::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;

            ctx.ip
                .ok_or_else(|| mlua::Error::runtime("VM has no IP address"))?;

            let local_port = ctx
                .alloc_ephemeral_port()
                .ok_or_else(|| mlua::Error::runtime("No ephemeral port available"))?;

            let connection_id = ctx.next_connection_id;
            ctx.next_connection_id = ctx.next_connection_id.saturating_add(1);
            let current_pid = ctx.current_pid;

            ctx.connections.insert(
                connection_id,
                ConnectionState {
                    local_port,
                    remote_ip,
                    remote_port,
                    pid: current_pid,
                    inbound: VecDeque::new(),
                },
            );
            ctx.pending_ephemeral_register.push(local_port);

            let conn_table = lua.create_table()?;
            conn_table.set("connection_id", connection_id)?;
            let methods = lua.create_table()?;
            methods.set(
                "send",
                lua.create_function(|lua, (conn_self, data): (mlua::Table, String)| {
                    let id: u64 = conn_self.get("connection_id")?;
                    let mut ctx = lua
                        .app_data_mut::<VmContext>()
                        .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                    let (local_port, remote_ip, remote_port) = {
                        let conn = ctx
                            .connections
                            .get(&id)
                            .ok_or_else(|| mlua::Error::runtime("Connection closed"))?;
                        (conn.local_port, conn.remote_ip, conn.remote_port)
                    };
                    let src_ip = ctx
                        .ip
                        .ok_or_else(|| mlua::Error::runtime("VM has no IP address"))?;
                    ctx.net_outbound.push(Packet::tcp(
                        src_ip,
                        local_port,
                        remote_ip,
                        remote_port,
                        data.into_bytes(),
                    ));
                    Ok(())
                })?,
            )?;
            methods.set(
                "recv",
                lua.create_function(|lua, conn_self: mlua::Table| {
                    let id: u64 = conn_self.get("connection_id")?;
                    let mut ctx = lua
                        .app_data_mut::<VmContext>()
                        .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                    let conn = ctx.connections.get_mut(&id).ok_or_else(|| {
                        mlua::Error::runtime("Connection closed")
                    })?;
                    match conn.inbound.pop_front() {
                        Some(pkt) => {
                            let t = lua.create_table()?;
                            t.set("src_ip", pkt.src_ip.to_string())?;
                            t.set("src_port", pkt.src_port)?;
                            t.set("dst_port", pkt.dst_port)?;
                            t.set("data", pkt.payload_str().unwrap_or("").to_string())?;
                            Ok(mlua::Value::Table(t))
                        }
                        None => Ok(mlua::Value::Nil),
                    }
                })?,
            )?;
            methods.set(
                "close",
                lua.create_function(|lua, conn_self: mlua::Table| {
                    let id: u64 = conn_self.get("connection_id")?;
                    let mut ctx = lua
                        .app_data_mut::<VmContext>()
                        .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                    if let Some(conn) = ctx.connections.remove(&id) {
                        ctx.pending_ephemeral_unregister.push(conn.local_port);
                    }
                    Ok(())
                })?,
            )?;
            let _ = conn_table.set_metatable(Some(lua.create_table_from([("__index", methods)])?));
            Ok(mlua::Value::Table(conn_table))
        })?,
    )?;

    lua.globals().set("net", net)?;
    Ok(())
}
