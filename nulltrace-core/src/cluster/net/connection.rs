#![allow(dead_code)]

use super::ip::Ipv4Addr;
use super::packet::Packet;
use std::collections::VecDeque;

/// Maximum packets queued per connection (oldest dropped when exceeded)
pub const MAX_INBOUND_PACKETS_PER_CONNECTION: usize = 128;

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

impl ConnectionState {
    /// Push a packet to inbound queue, dropping oldest if over limit
    pub fn push_inbound(&mut self, packet: Packet) {
        if self.inbound.len() >= MAX_INBOUND_PACKETS_PER_CONNECTION {
            // Drop oldest packet (head-drop policy)
            self.inbound.pop_front();
        }
        self.inbound.push_back(packet);
    }
}
