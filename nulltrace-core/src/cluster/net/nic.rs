#![allow(dead_code)]

use super::ip::{Ipv4Addr, Subnet};
use super::packet::Packet;
use std::collections::VecDeque;

/// A simulated Network Interface Card (NIC).
///
/// Each VM gets one NIC. It holds the VM's IP address, subnet configuration,
/// and packet buffers for inbound/outbound traffic.
pub struct NIC {
    pub ip: Ipv4Addr,
    pub subnet: Subnet,
    pub gateway: Ipv4Addr,
    pub mac: [u8; 6],
    pub inbound: VecDeque<Packet>,
    pub outbound: VecDeque<Packet>,
    /// Ports this NIC is listening on (for incoming connections).
    pub listening_ports: Vec<u16>,
}

impl NIC {
    /// Create a new NIC with the given IP and subnet.
    pub fn new(ip: Ipv4Addr, subnet: Subnet) -> Self {
        let gateway = subnet.gateway();
        let mac = Self::generate_mac(&ip);

        Self {
            ip,
            subnet,
            gateway,
            mac,
            inbound: VecDeque::new(),
            outbound: VecDeque::new(),
            listening_ports: Vec::new(),
        }
    }

    /// Create a NIC with an auto-allocated IP from the subnet.
    pub fn from_subnet(subnet: &mut Subnet) -> Option<Self> {
        let ip = subnet.allocate_next()?;
        Some(Self::new(ip, subnet.clone()))
    }

    /// Generate a deterministic MAC address from an IP (for simplicity).
    fn generate_mac(ip: &Ipv4Addr) -> [u8; 6] {
        let o = ip.octets();
        [0x02, 0x00, o[0], o[1], o[2], o[3]] // Locally administered MAC
    }

    /// Queue a packet for sending.
    pub fn send(&mut self, packet: Packet) {
        self.outbound.push_back(packet);
    }

    /// Receive the next inbound packet, if any.
    pub fn recv(&mut self) -> Option<Packet> {
        self.inbound.pop_front()
    }

    /// Deliver a packet to this NIC's inbound buffer.
    pub fn deliver(&mut self, packet: Packet) {
        self.inbound.push_back(packet);
    }

    /// Drain all outbound packets (called by the router/netmanager each tick).
    pub fn drain_outbound(&mut self) -> Vec<Packet> {
        self.outbound.drain(..).collect()
    }

    /// Check if this NIC has any pending inbound packets.
    pub fn has_inbound(&self) -> bool {
        !self.inbound.is_empty()
    }

    /// Check if this NIC has any pending outbound packets.
    pub fn has_outbound(&self) -> bool {
        !self.outbound.is_empty()
    }

    /// Start listening on a port.
    pub fn listen(&mut self, port: u16) {
        if !self.listening_ports.contains(&port) {
            self.listening_ports.push(port);
        }
    }

    /// Stop listening on a port.
    pub fn unlisten(&mut self, port: u16) {
        self.listening_ports.retain(|&p| p != port);
    }

    /// Check if this NIC is listening on a specific port.
    pub fn is_listening(&self, port: u16) -> bool {
        self.listening_ports.contains(&port)
    }

    /// Format the MAC address as a string.
    pub fn mac_string(&self) -> String {
        format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.mac[0], self.mac[1], self.mac[2], self.mac[3], self.mac[4], self.mac[5]
        )
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_subnet() -> Subnet {
        Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 24)
    }

    #[test]
    fn test_nic_creation() {
        let nic = NIC::new(Ipv4Addr::new(10, 0, 1, 2), test_subnet());

        assert_eq!(nic.ip, Ipv4Addr::new(10, 0, 1, 2));
        assert_eq!(nic.gateway, Ipv4Addr::new(10, 0, 1, 1));
        assert_eq!(nic.mac[0], 0x02); // Locally administered
    }

    #[test]
    fn test_nic_from_subnet() {
        let mut subnet = test_subnet();
        let nic = NIC::from_subnet(&mut subnet).unwrap();

        assert_eq!(nic.ip, Ipv4Addr::new(10, 0, 1, 2)); // First allocated
    }

    #[test]
    fn test_nic_send_recv() {
        let mut nic = NIC::new(Ipv4Addr::new(10, 0, 1, 2), test_subnet());

        // Deliver a packet
        let pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 3),
            1234,
            Ipv4Addr::new(10, 0, 1, 2),
            80,
            b"hello".to_vec(),
        );
        nic.deliver(pkt);

        assert!(nic.has_inbound());
        let received = nic.recv().unwrap();
        assert_eq!(received.payload_str(), Some("hello"));
        assert!(!nic.has_inbound());
    }

    #[test]
    fn test_nic_outbound_drain() {
        let mut nic = NIC::new(Ipv4Addr::new(10, 0, 1, 2), test_subnet());

        nic.send(Packet::icmp_echo(
            Ipv4Addr::new(10, 0, 1, 2),
            Ipv4Addr::new(10, 0, 1, 3),
        ));
        nic.send(Packet::icmp_echo(
            Ipv4Addr::new(10, 0, 1, 2),
            Ipv4Addr::new(10, 0, 1, 4),
        ));

        let drained = nic.drain_outbound();
        assert_eq!(drained.len(), 2);
        assert!(!nic.has_outbound());
    }

    #[test]
    fn test_nic_listening() {
        let mut nic = NIC::new(Ipv4Addr::new(10, 0, 1, 2), test_subnet());

        assert!(!nic.is_listening(80));
        nic.listen(80);
        assert!(nic.is_listening(80));
        nic.unlisten(80);
        assert!(!nic.is_listening(80));
    }

    #[test]
    fn test_mac_string() {
        let nic = NIC::new(Ipv4Addr::new(10, 0, 1, 2), test_subnet());
        let mac = nic.mac_string();
        assert_eq!(mac, "02:00:0a:00:01:02");
    }
}
