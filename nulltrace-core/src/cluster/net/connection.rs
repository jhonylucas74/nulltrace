#![allow(dead_code)]

use super::ip::Ipv4Addr;
use super::packet::Packet;
use std::collections::VecDeque;

/// State for a single connection (net.connect). Owned by a process (pid).
/// Ephemeral port is bound to this connection; not in LISTEN.
#[derive(Clone, Debug)]
pub struct ConnectionState {
    pub local_port: u16,
    pub remote_ip: Ipv4Addr,
    pub remote_port: u16,
    pub pid: u64,
    pub inbound: VecDeque<Packet>,
}
