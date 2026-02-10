#![allow(dead_code)]

use super::ip::Ipv4Addr;
use std::fmt;

/// Network protocol types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Protocol {
    TCP,
    UDP,
    ICMP,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::TCP => write!(f, "TCP"),
            Protocol::UDP => write!(f, "UDP"),
            Protocol::ICMP => write!(f, "ICMP"),
        }
    }
}

/// A simulated network packet.
#[derive(Clone, Debug)]
pub struct Packet {
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: Protocol,
    pub ttl: u8,
    pub payload: Vec<u8>,
}

impl Packet {
    /// Create a new TCP packet.
    pub fn tcp(src_ip: Ipv4Addr, src_port: u16, dst_ip: Ipv4Addr, dst_port: u16, payload: Vec<u8>) -> Self {
        Self {
            src_ip,
            dst_ip,
            src_port,
            dst_port,
            protocol: Protocol::TCP,
            ttl: 64,
            payload,
        }
    }

    /// Create a new UDP packet.
    pub fn udp(src_ip: Ipv4Addr, src_port: u16, dst_ip: Ipv4Addr, dst_port: u16, payload: Vec<u8>) -> Self {
        Self {
            src_ip,
            dst_ip,
            src_port,
            dst_port,
            protocol: Protocol::UDP,
            ttl: 64,
            payload,
        }
    }

    /// Create an ICMP echo request (ping).
    pub fn icmp_echo(src_ip: Ipv4Addr, dst_ip: Ipv4Addr) -> Self {
        Self {
            src_ip,
            dst_ip,
            src_port: 0,
            dst_port: 0,
            protocol: Protocol::ICMP,
            ttl: 64,
            payload: vec![],
        }
    }

    /// Create an ICMP echo reply (pong).
    pub fn icmp_reply(original: &Packet) -> Self {
        Self {
            src_ip: original.dst_ip,
            dst_ip: original.src_ip,
            src_port: 0,
            dst_port: 0,
            protocol: Protocol::ICMP,
            ttl: 64,
            payload: vec![],
        }
    }

    /// Decrement TTL. Returns false if TTL reached 0 (packet should be dropped).
    pub fn decrement_ttl(&mut self) -> bool {
        if self.ttl == 0 {
            return false;
        }
        self.ttl -= 1;
        self.ttl > 0
    }

    /// Get the payload as a UTF-8 string (if valid).
    pub fn payload_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.payload).ok()
    }

    /// Size of the packet in bytes (header + payload).
    pub fn size(&self) -> usize {
        // Simulated header: 20 bytes IP + 8/20 bytes transport
        let header = match self.protocol {
            Protocol::TCP => 40,  // 20 IP + 20 TCP
            Protocol::UDP => 28,  // 20 IP + 8 UDP
            Protocol::ICMP => 28, // 20 IP + 8 ICMP
        };
        header + self.payload.len()
    }
}

impl fmt::Display for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}:{} → {}:{} (TTL={}, {}B)",
            self.protocol,
            self.src_ip,
            self.src_port,
            self.dst_ip,
            self.dst_port,
            self.ttl,
            self.size(),
        )
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_packet() {
        let pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 2),
            12345,
            Ipv4Addr::new(10, 0, 2, 3),
            80,
            b"GET / HTTP/1.1\r\n".to_vec(),
        );

        assert_eq!(pkt.protocol, Protocol::TCP);
        assert_eq!(pkt.src_port, 12345);
        assert_eq!(pkt.dst_port, 80);
        assert_eq!(pkt.ttl, 64);
        assert_eq!(pkt.payload_str(), Some("GET / HTTP/1.1\r\n"));
    }

    #[test]
    fn test_icmp_echo_reply() {
        let ping = Packet::icmp_echo(
            Ipv4Addr::new(10, 0, 1, 2),
            Ipv4Addr::new(10, 0, 1, 3),
        );
        let pong = Packet::icmp_reply(&ping);

        assert_eq!(pong.src_ip, Ipv4Addr::new(10, 0, 1, 3));
        assert_eq!(pong.dst_ip, Ipv4Addr::new(10, 0, 1, 2));
        assert_eq!(pong.protocol, Protocol::ICMP);
    }

    #[test]
    fn test_ttl_decrement() {
        let mut pkt = Packet::icmp_echo(
            Ipv4Addr::new(10, 0, 1, 2),
            Ipv4Addr::new(10, 0, 1, 3),
        );

        assert_eq!(pkt.ttl, 64);
        assert!(pkt.decrement_ttl());
        assert_eq!(pkt.ttl, 63);

        // Simulate TTL exhaustion
        pkt.ttl = 1;
        assert!(!pkt.decrement_ttl()); // TTL=0 → drop
    }

    #[test]
    fn test_packet_size() {
        let pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 2),
            1234,
            Ipv4Addr::new(10, 0, 2, 3),
            80,
            vec![0u8; 100],
        );

        assert_eq!(pkt.size(), 140); // 40 header + 100 payload
    }

    #[test]
    fn test_packet_display() {
        let pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 2),
            1234,
            Ipv4Addr::new(10, 0, 2, 3),
            80,
            vec![],
        );

        let s = pkt.to_string();
        assert!(s.contains("TCP"));
        assert!(s.contains("10.0.1.2:1234"));
        assert!(s.contains("10.0.2.3:80"));
    }
}
