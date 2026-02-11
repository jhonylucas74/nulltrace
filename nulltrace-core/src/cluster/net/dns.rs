#![allow(dead_code)]

use super::ip::Ipv4Addr;
use std::collections::HashMap;

/// DNS record types.
#[derive(Clone, Debug, PartialEq)]
pub enum DnsRecordType {
    A,     // hostname → IP
    PTR,   // IP → hostname (reverse)
    CNAME, // alias → canonical name
    MX,    // mail server
}

/// A single DNS record.
#[derive(Clone, Debug)]
pub struct DnsRecord {
    pub name: String,
    pub record_type: DnsRecordType,
    pub value: String,
    pub ttl: u32,
}

/// DNS resolver backed by in-memory HashMap.
/// Can be extended to use Redis as cache layer later.
pub struct DnsResolver {
    a_records: HashMap<String, Ipv4Addr>,
    ptr_records: HashMap<String, String>, // "10.0.1.2" → hostname
    cname_records: HashMap<String, String>,
}

impl DnsResolver {
    pub fn new() -> Self {
        Self {
            a_records: HashMap::new(),
            ptr_records: HashMap::new(),
            cname_records: HashMap::new(),
        }
    }

    /// Resolve a hostname to an IP address (A record).
    /// Follows CNAME chains (max 10 hops to prevent loops).
    pub fn resolve(&self, hostname: &str) -> Option<Ipv4Addr> {
        let mut current = hostname.to_string();

        for _ in 0..10 {
            // Check A record
            if let Some(ip) = self.a_records.get(&current) {
                return Some(*ip);
            }

            // Check CNAME and follow
            if let Some(canonical) = self.cname_records.get(&current) {
                current = canonical.clone();
                continue;
            }

            return None;
        }

        None // CNAME chain too long
    }

    /// Register an A record (hostname → IP).
    /// Also auto-registers the reverse PTR record.
    pub fn register_a(&mut self, hostname: &str, ip: Ipv4Addr) {
        self.a_records.insert(hostname.to_string(), ip);
        self.ptr_records.insert(ip.to_string(), hostname.to_string());
    }

    /// Remove an A record and its PTR.
    pub fn unregister_a(&mut self, hostname: &str) {
        if let Some(ip) = self.a_records.remove(hostname) {
            self.ptr_records.remove(&ip.to_string());
        }
    }

    /// Reverse DNS lookup (IP → hostname).
    pub fn reverse_lookup(&self, ip: Ipv4Addr) -> Option<&str> {
        self.ptr_records.get(&ip.to_string()).map(|s| s.as_str())
    }

    /// Register a CNAME alias.
    pub fn register_cname(&mut self, alias: &str, canonical: &str) {
        self.cname_records.insert(alias.to_string(), canonical.to_string());
    }

    /// Resolve a CNAME to its canonical hostname.
    pub fn resolve_cname(&self, alias: &str) -> Option<&str> {
        self.cname_records.get(alias).map(|s| s.as_str())
    }

    /// Get all registered A records.
    pub fn all_a_records(&self) -> &HashMap<String, Ipv4Addr> {
        &self.a_records
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_resolve() {
        let mut dns = DnsResolver::new();
        let ip = Ipv4Addr::new(10, 0, 1, 10);

        dns.register_a("web-srv.internal", ip);

        assert_eq!(dns.resolve("web-srv.internal"), Some(ip));
    }

    #[test]
    fn test_reverse_lookup() {
        let mut dns = DnsResolver::new();
        let ip = Ipv4Addr::new(10, 0, 1, 10);

        dns.register_a("db-prod.internal", ip);

        assert_eq!(dns.reverse_lookup(ip), Some("db-prod.internal"));
    }

    #[test]
    fn test_cname_resolution() {
        let mut dns = DnsResolver::new();
        let ip = Ipv4Addr::new(10, 0, 1, 10);

        dns.register_a("web-srv-01.internal", ip);
        dns.register_cname("www.internal", "web-srv-01.internal");

        // CNAME → A record
        assert_eq!(dns.resolve("www.internal"), Some(ip));
    }

    #[test]
    fn test_cname_chain() {
        let mut dns = DnsResolver::new();
        let ip = Ipv4Addr::new(10, 0, 1, 10);

        dns.register_a("real-host.internal", ip);
        dns.register_cname("alias1.internal", "real-host.internal");
        dns.register_cname("alias2.internal", "alias1.internal");

        assert_eq!(dns.resolve("alias2.internal"), Some(ip));
    }

    #[test]
    fn test_unresolved_returns_none() {
        let dns = DnsResolver::new();
        assert_eq!(dns.resolve("nonexistent.internal"), None);
    }

    #[test]
    fn test_unregister() {
        let mut dns = DnsResolver::new();
        let ip = Ipv4Addr::new(10, 0, 1, 10);

        dns.register_a("temp.internal", ip);
        assert!(dns.resolve("temp.internal").is_some());

        dns.unregister_a("temp.internal");
        assert!(dns.resolve("temp.internal").is_none());
        assert!(dns.reverse_lookup(ip).is_none());
    }
}
