#![allow(dead_code)]

use std::fmt;

/// A simulated IPv4 address.
#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub struct Ipv4Addr {
    octets: [u8; 4],
}

impl Ipv4Addr {
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self {
            octets: [a, b, c, d],
        }
    }

    pub const fn octets(&self) -> [u8; 4] {
        self.octets
    }

    /// Convert to a 32-bit unsigned integer for arithmetic operations.
    pub const fn to_u32(&self) -> u32 {
        ((self.octets[0] as u32) << 24)
            | ((self.octets[1] as u32) << 16)
            | ((self.octets[2] as u32) << 8)
            | (self.octets[3] as u32)
    }

    /// Create from a 32-bit unsigned integer.
    pub const fn from_u32(val: u32) -> Self {
        Self {
            octets: [
                ((val >> 24) & 0xFF) as u8,
                ((val >> 16) & 0xFF) as u8,
                ((val >> 8) & 0xFF) as u8,
                (val & 0xFF) as u8,
            ],
        }
    }

    /// Parse from a dotted-decimal string like "10.0.1.2".
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return None;
        }

        let a = parts[0].parse::<u8>().ok()?;
        let b = parts[1].parse::<u8>().ok()?;
        let c = parts[2].parse::<u8>().ok()?;
        let d = parts[3].parse::<u8>().ok()?;

        Some(Self::new(a, b, c, d))
    }

    /// Check if this is a private (RFC 1918) address.
    pub fn is_private(&self) -> bool {
        match self.octets[0] {
            10 => true,                                          // 10.0.0.0/8
            172 => (16..=31).contains(&self.octets[1]),          // 172.16.0.0/12
            192 => self.octets[1] == 168,                        // 192.168.0.0/16
            _ => false,
        }
    }

    /// Check if this is a loopback address (127.x.x.x).
    pub fn is_loopback(&self) -> bool {
        self.octets[0] == 127
    }

    /// Check if this is a broadcast address within a given subnet.
    pub fn is_broadcast_in(&self, subnet: &Subnet) -> bool {
        *self == subnet.broadcast()
    }
}

impl fmt::Display for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.octets[0], self.octets[1], self.octets[2], self.octets[3]
        )
    }
}

/// A subnet defined by a network address and prefix length.
///
/// Example: 10.0.1.0/24 means addresses 10.0.1.0–10.0.1.255
#[derive(Clone, Debug)]
pub struct Subnet {
    network: Ipv4Addr,
    prefix: u8,
    /// Tracks the next available host address for auto-allocation.
    next_host: u32,
}

impl Subnet {
    /// Create a new subnet. Panics if prefix > 32.
    pub fn new(network: Ipv4Addr, prefix: u8) -> Self {
        assert!(prefix <= 32, "Prefix must be 0–32");

        // Mask the network address to ensure it's valid
        let mask = Self::prefix_to_mask(prefix);
        let masked_network = Ipv4Addr::from_u32(network.to_u32() & mask);

        Self {
            network: masked_network,
            prefix,
            next_host: masked_network.to_u32() + 2, // .1 is gateway, start at .2
        }
    }

    /// Parse from CIDR notation like "10.0.1.0/24".
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return None;
        }

        let network = Ipv4Addr::parse(parts[0])?;
        let prefix = parts[1].parse::<u8>().ok()?;

        if prefix > 32 {
            return None;
        }

        Some(Self::new(network, prefix))
    }

    /// The subnet mask as a u32 (e.g., /24 → 0xFFFFFF00).
    const fn prefix_to_mask(prefix: u8) -> u32 {
        if prefix == 0 {
            0
        } else {
            !0u32 << (32 - prefix)
        }
    }

    /// Get the subnet mask as an Ipv4Addr.
    pub fn mask(&self) -> Ipv4Addr {
        Ipv4Addr::from_u32(Self::prefix_to_mask(self.prefix))
    }

    /// The network address (e.g., 10.0.1.0).
    pub fn network(&self) -> Ipv4Addr {
        self.network
    }

    /// The prefix length (e.g., 24).
    pub fn prefix(&self) -> u8 {
        self.prefix
    }

    /// The gateway address — by convention, .1 in the subnet.
    pub fn gateway(&self) -> Ipv4Addr {
        Ipv4Addr::from_u32(self.network.to_u32() + 1)
    }

    /// The broadcast address — last address in the subnet.
    pub fn broadcast(&self) -> Ipv4Addr {
        let mask = Self::prefix_to_mask(self.prefix);
        Ipv4Addr::from_u32(self.network.to_u32() | !mask)
    }

    /// Check if an IP address belongs to this subnet.
    pub fn contains(&self, addr: Ipv4Addr) -> bool {
        let mask = Self::prefix_to_mask(self.prefix);
        (addr.to_u32() & mask) == self.network.to_u32()
    }

    /// Total number of usable host addresses (excludes network + broadcast).
    pub fn host_count(&self) -> u32 {
        if self.prefix >= 31 {
            return 0;
        }
        (1u32 << (32 - self.prefix)) - 2
    }

    /// Allocate the next available IP in this subnet.
    /// Returns None if the subnet is exhausted.
    pub fn allocate_next(&mut self) -> Option<Ipv4Addr> {
        let broadcast = self.broadcast().to_u32();

        if self.next_host >= broadcast {
            return None; // Subnet is full
        }

        let addr = Ipv4Addr::from_u32(self.next_host);
        self.next_host += 1;
        Some(addr)
    }
}

impl fmt::Display for Subnet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.network, self.prefix)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv4_new_and_display() {
        let ip = Ipv4Addr::new(10, 0, 1, 2);
        assert_eq!(ip.to_string(), "10.0.1.2");
        assert_eq!(ip.octets(), [10, 0, 1, 2]);
    }

    #[test]
    fn test_ipv4_parse() {
        let ip = Ipv4Addr::parse("192.168.1.100").unwrap();
        assert_eq!(ip, Ipv4Addr::new(192, 168, 1, 100));

        assert!(Ipv4Addr::parse("invalid").is_none());
        assert!(Ipv4Addr::parse("256.0.0.1").is_none());
        assert!(Ipv4Addr::parse("10.0.1").is_none());
    }

    #[test]
    fn test_ipv4_u32_roundtrip() {
        let ip = Ipv4Addr::new(10, 0, 1, 2);
        let val = ip.to_u32();
        assert_eq!(Ipv4Addr::from_u32(val), ip);
    }

    #[test]
    fn test_ipv4_private() {
        assert!(Ipv4Addr::new(10, 0, 0, 1).is_private());
        assert!(Ipv4Addr::new(172, 16, 0, 1).is_private());
        assert!(Ipv4Addr::new(192, 168, 1, 1).is_private());
        assert!(!Ipv4Addr::new(8, 8, 8, 8).is_private());
    }

    #[test]
    fn test_subnet_basics() {
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 24);

        assert_eq!(subnet.network(), Ipv4Addr::new(10, 0, 1, 0));
        assert_eq!(subnet.gateway(), Ipv4Addr::new(10, 0, 1, 1));
        assert_eq!(subnet.broadcast(), Ipv4Addr::new(10, 0, 1, 255));
        assert_eq!(subnet.mask(), Ipv4Addr::new(255, 255, 255, 0));
        assert_eq!(subnet.host_count(), 254);
    }

    #[test]
    fn test_subnet_contains() {
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 24);

        assert!(subnet.contains(Ipv4Addr::new(10, 0, 1, 1)));
        assert!(subnet.contains(Ipv4Addr::new(10, 0, 1, 254)));
        assert!(!subnet.contains(Ipv4Addr::new(10, 0, 2, 1)));
        assert!(!subnet.contains(Ipv4Addr::new(192, 168, 1, 1)));
    }

    #[test]
    fn test_subnet_parse() {
        let subnet = Subnet::parse("10.0.1.0/24").unwrap();
        assert_eq!(subnet.network(), Ipv4Addr::new(10, 0, 1, 0));
        assert_eq!(subnet.prefix(), 24);

        assert!(Subnet::parse("invalid").is_none());
        assert!(Subnet::parse("10.0.0.0/33").is_none());
    }

    #[test]
    fn test_subnet_allocate() {
        let mut subnet = Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 24);

        // First allocation should be .2 (gateway is .1)
        assert_eq!(subnet.allocate_next(), Some(Ipv4Addr::new(10, 0, 1, 2)));
        assert_eq!(subnet.allocate_next(), Some(Ipv4Addr::new(10, 0, 1, 3)));
        assert_eq!(subnet.allocate_next(), Some(Ipv4Addr::new(10, 0, 1, 4)));
    }

    #[test]
    fn test_subnet_allocate_exhaustion() {
        // /30 subnet: only 2 usable hosts (.1 is gateway, .2 and .3 are usable, .4 is broadcast... wait)
        // /30 = 4 addresses total: .0 (network), .1 (gateway), .2 (host), .3 (broadcast)
        let mut subnet = Subnet::new(Ipv4Addr::new(10, 0, 0, 0), 30);

        assert_eq!(subnet.allocate_next(), Some(Ipv4Addr::new(10, 0, 0, 2)));
        assert_eq!(subnet.allocate_next(), None); // .3 is broadcast
    }

    #[test]
    fn test_subnet_masking() {
        // If someone passes 10.0.1.50/24, it should be normalized to 10.0.1.0/24
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 1, 50), 24);
        assert_eq!(subnet.network(), Ipv4Addr::new(10, 0, 1, 0));
    }
}
