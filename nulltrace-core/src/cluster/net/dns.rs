#![allow(dead_code)]

use super::ip::Ipv4Addr;

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

/// DNS resolver backed by Redis.
///
/// Redis key schema:
///   dns:a:{hostname}       → IP address string
///   dns:ptr:{ip}           → hostname string
///   dns:cname:{alias}      → canonical hostname
///   dns:mx:{domain}        → mail server hostname
///   dns:zone:{domain}      → JSON list of all records
///
/// TODO: Implement Redis connection in Phase 3.
pub struct DnsResolver {
    // Will hold redis::Client when Phase 3 is implemented
}

impl DnsResolver {
    pub fn new() -> Self {
        Self {}
    }

    /// Resolve a hostname to an IP address (A record).
    /// Redis key: `dns:a:{hostname}`
    pub fn resolve(&self, _hostname: &str) -> Option<Ipv4Addr> {
        // TODO: Phase 3 — query Redis
        None
    }

    /// Register an A record.
    /// Redis key: `dns:a:{hostname}` = ip
    pub fn register_a(&self, _hostname: &str, _ip: Ipv4Addr) {
        // TODO: Phase 3 — write to Redis
    }

    /// Reverse DNS lookup.
    /// Redis key: `dns:ptr:{ip}`
    pub fn reverse_lookup(&self, _ip: Ipv4Addr) -> Option<String> {
        // TODO: Phase 3 — query Redis
        None
    }

    /// Register a PTR record (reverse DNS).
    pub fn register_ptr(&self, _ip: Ipv4Addr, _hostname: &str) {
        // TODO: Phase 3 — write to Redis
    }

    /// Register a CNAME alias.
    pub fn register_cname(&self, _alias: &str, _canonical: &str) {
        // TODO: Phase 3 — write to Redis
    }

    /// Resolve a CNAME to its canonical hostname.
    pub fn resolve_cname(&self, _alias: &str) -> Option<String> {
        // TODO: Phase 3 — query Redis
        None
    }
}
