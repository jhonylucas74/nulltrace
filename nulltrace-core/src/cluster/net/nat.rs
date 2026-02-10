#![allow(dead_code)]

use super::ip::{Ipv4Addr, Subnet};
use super::packet::Packet;
use std::collections::HashMap;

/// Result of a NAT lookup.
pub enum NatAction {
    /// Packet was translated — use the new IP and port.
    Translated(Ipv4Addr, u16),
    /// No matching NAT rule.
    NoMatch,
}

/// Source NAT rule — rewrites the source IP for outbound traffic.
///
/// Example: VMs in 10.0.1.0/24 appear as 203.0.113.1 when leaving the network.
#[derive(Clone, Debug)]
pub struct SnatRule {
    pub internal_subnet: Subnet,
    pub external_ip: Ipv4Addr,
}

/// Destination NAT rule — port forwarding from external to internal.
///
/// Example: Traffic to 203.0.113.1:80 is forwarded to 10.0.1.2:8080.
#[derive(Clone, Debug)]
pub struct DnatRule {
    pub external_ip: Ipv4Addr,
    pub external_port: u16,
    pub internal_ip: Ipv4Addr,
    pub internal_port: u16,
}

/// Key for tracking active NAT connections (for reverse mapping).
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct NatKey {
    pub src_ip: Ipv4Addr,
    pub src_port: u16,
    pub dst_ip: Ipv4Addr,
    pub dst_port: u16,
}

/// Mapping of an active NAT translation.
#[derive(Clone, Debug)]
pub struct NatMapping {
    pub original_src_ip: Ipv4Addr,
    pub translated_src_ip: Ipv4Addr,
}

/// The NAT table — manages SNAT, DNAT, and active connection tracking.
pub struct NatTable {
    pub snat_rules: Vec<SnatRule>,
    pub dnat_rules: Vec<DnatRule>,
    pub connections: HashMap<NatKey, NatMapping>,
}

impl NatTable {
    pub fn new() -> Self {
        Self {
            snat_rules: Vec::new(),
            dnat_rules: Vec::new(),
            connections: HashMap::new(),
        }
    }

    /// Add a source NAT rule.
    pub fn add_snat(&mut self, internal_subnet: Subnet, external_ip: Ipv4Addr) {
        self.snat_rules.push(SnatRule {
            internal_subnet,
            external_ip,
        });
    }

    /// Add a destination NAT (port forwarding) rule.
    pub fn add_dnat(
        &mut self,
        external_ip: Ipv4Addr,
        external_port: u16,
        internal_ip: Ipv4Addr,
        internal_port: u16,
    ) {
        self.dnat_rules.push(DnatRule {
            external_ip,
            external_port,
            internal_ip,
            internal_port,
        });
    }

    /// Apply SNAT — check if the packet's source IP matches any SNAT rule.
    pub fn apply_snat(&self, packet: &Packet) -> NatAction {
        for rule in &self.snat_rules {
            if rule.internal_subnet.contains(packet.src_ip) {
                return NatAction::Translated(rule.external_ip, 0);
            }
        }
        NatAction::NoMatch
    }

    /// Apply DNAT — check if the packet matches any port-forwarding rule.
    pub fn apply_dnat(&self, packet: &Packet) -> NatAction {
        for rule in &self.dnat_rules {
            if packet.dst_ip == rule.external_ip && packet.dst_port == rule.external_port {
                return NatAction::Translated(rule.internal_ip, rule.internal_port);
            }
        }

        // Also check reverse mappings for return traffic
        let key = NatKey {
            src_ip: packet.src_ip,
            src_port: packet.src_port,
            dst_ip: packet.dst_ip,
            dst_port: packet.dst_port,
        };

        if let Some(mapping) = self.connections.get(&key) {
            return NatAction::Translated(mapping.original_src_ip, packet.dst_port);
        }

        NatAction::NoMatch
    }

    /// Track an active NAT connection (for reverse mapping of return packets).
    pub fn track_connection(&mut self, original_packet: &Packet, translated_src: Ipv4Addr) {
        let key = NatKey {
            src_ip: original_packet.dst_ip,
            src_port: original_packet.dst_port,
            dst_ip: translated_src,
            dst_port: original_packet.src_port,
        };

        self.connections.insert(
            key,
            NatMapping {
                original_src_ip: original_packet.src_ip,
                translated_src_ip: translated_src,
            },
        );
    }

    /// Remove stale connections (could be called periodically).
    pub fn clear_connections(&mut self) {
        self.connections.clear();
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snat() {
        let mut nat = NatTable::new();
        nat.add_snat(
            Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 24),
            Ipv4Addr::new(203, 0, 113, 1),
        );

        let pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 5),
            12345,
            Ipv4Addr::new(8, 8, 8, 8),
            53,
            vec![],
        );

        match nat.apply_snat(&pkt) {
            NatAction::Translated(ip, _) => {
                assert_eq!(ip, Ipv4Addr::new(203, 0, 113, 1));
            }
            NatAction::NoMatch => panic!("Expected SNAT match"),
        }
    }

    #[test]
    fn test_snat_no_match() {
        let mut nat = NatTable::new();
        nat.add_snat(
            Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 24),
            Ipv4Addr::new(203, 0, 113, 1),
        );

        let pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 2, 5), // Different subnet
            12345,
            Ipv4Addr::new(8, 8, 8, 8),
            53,
            vec![],
        );

        match nat.apply_snat(&pkt) {
            NatAction::NoMatch => {} // Expected
            _ => panic!("Expected no SNAT match"),
        }
    }

    #[test]
    fn test_dnat_port_forwarding() {
        let mut nat = NatTable::new();
        nat.add_dnat(
            Ipv4Addr::new(203, 0, 113, 1), // external
            80,
            Ipv4Addr::new(10, 0, 1, 5),    // internal
            8080,
        );

        let pkt = Packet::tcp(
            Ipv4Addr::new(192, 168, 1, 100),
            54321,
            Ipv4Addr::new(203, 0, 113, 1),
            80,
            vec![],
        );

        match nat.apply_dnat(&pkt) {
            NatAction::Translated(ip, port) => {
                assert_eq!(ip, Ipv4Addr::new(10, 0, 1, 5));
                assert_eq!(port, 8080);
            }
            NatAction::NoMatch => panic!("Expected DNAT match"),
        }
    }

    #[test]
    fn test_connection_tracking() {
        let mut nat = NatTable::new();
        nat.add_snat(
            Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 24),
            Ipv4Addr::new(203, 0, 113, 1),
        );

        // Original outbound packet: 10.0.1.5:12345 → 8.8.8.8:53
        let original = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 5),
            12345,
            Ipv4Addr::new(8, 8, 8, 8),
            53,
            vec![],
        );

        // Track the SNAT translation
        nat.track_connection(&original, Ipv4Addr::new(203, 0, 113, 1));

        // Return packet: 8.8.8.8:53 → 203.0.113.1:12345
        let reply = Packet::tcp(
            Ipv4Addr::new(8, 8, 8, 8),
            53,
            Ipv4Addr::new(203, 0, 113, 1),
            12345,
            vec![],
        );

        match nat.apply_dnat(&reply) {
            NatAction::Translated(ip, _) => {
                assert_eq!(ip, Ipv4Addr::new(10, 0, 1, 5)); // Correctly mapped back
            }
            NatAction::NoMatch => panic!("Expected connection tracking match"),
        }
    }
}
