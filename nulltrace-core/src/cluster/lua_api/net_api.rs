#![allow(dead_code)]

use super::context::VmContext;
use crate::net::ip::Ipv4Addr;
use crate::net::packet::Packet;
use mlua::{Lua, Result};

/// Register the `net` table on the Lua state.
/// Network operations are in-memory (NIC buffers), no DB calls needed.
pub fn register(lua: &Lua) -> Result<()> {
    let net = lua.create_table()?;

    // net.send(ip, port, data) — queue a TCP packet to the outbound buffer
    net.set(
        "send",
        lua.create_function(|lua, (dst_ip_str, dst_port, data): (String, u16, String)| {
            let dst_ip = Ipv4Addr::parse(&dst_ip_str)
                .ok_or_else(|| mlua::Error::runtime(format!("Invalid IP: {}", dst_ip_str)))?;

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

    // net.listen(port)
    net.set(
        "listen",
        lua.create_function(|lua, port: u16| {
            let mut ctx = lua
                .app_data_mut::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;

            if !ctx.listening_ports.contains(&port) {
                ctx.listening_ports.push(port);
            }
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

    lua.globals().set("net", net)?;
    Ok(())
}
