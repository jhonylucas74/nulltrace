#![allow(dead_code)]

use super::ip::{Ipv4Addr, Subnet};
use super::nat::{NatTable, NatAction};
use super::packet::Packet;
use uuid::Uuid;

/// A network interface on a router — connects the router to a subnet.
#[derive(Clone, Debug)]
pub struct RouterInterface {
    pub ip: Ipv4Addr,
    pub subnet: Subnet,
}

/// Describes where to forward a packet.
#[derive(Clone, Debug)]
pub enum NextHop {
    /// Deliver directly to the destination — it's on a directly connected subnet.
    Direct(usize), // Index into the router's interfaces
    /// Forward to another router at this IP.
    Gateway(Ipv4Addr),
    /// Destination is on another pod — delegate to the NetManager.
    NetManager,
}

/// A single routing table entry.
#[derive(Clone, Debug)]
pub struct Route {
    pub destination: Subnet,
    pub next_hop: NextHop,
    pub metric: u8,
}

/// Result of routing a packet.
pub enum RouteResult {
    /// Deliver to a VM on a directly connected subnet.
    Deliver {
        interface_idx: usize,
        packet: Packet,
    },
    /// Forward to another router.
    Forward {
        next_hop_ip: Ipv4Addr,
        packet: Packet,
    },
    /// Send to NetManager for cross-pod delivery.
    CrossPod(Packet),
    /// Packet was dropped (no route, TTL expired, firewall).
    Drop(DropReason),
}

#[derive(Debug)]
pub enum DropReason {
    NoRoute,
    TtlExpired,
    Firewall,
}

/// Firewall rule — evaluated in order, first match wins.
#[derive(Clone, Debug)]
pub struct FirewallRule {
    pub src: Option<Subnet>,      // None = any source
    pub dst: Option<Subnet>,      // None = any destination
    pub dst_port: Option<u16>,    // None = any port
    pub action: FirewallAction,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FirewallAction {
    Accept,
    Drop,
}

/// A simulated router that connects subnets and forwards packets.
pub struct Router {
    pub id: Uuid,
    pub interfaces: Vec<RouterInterface>,
    pub routing_table: Vec<Route>,
    pub nat_table: NatTable,
    pub firewall: Vec<FirewallRule>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            interfaces: Vec::new(),
            routing_table: Vec::new(),
            nat_table: NatTable::new(),
            firewall: Vec::new(),
        }
    }

    /// Attach the router to a subnet.
    pub fn add_interface(&mut self, ip: Ipv4Addr, subnet: Subnet) -> usize {
        let idx = self.interfaces.len();
        self.interfaces.push(RouterInterface { ip, subnet: subnet.clone() });

        // Auto-add a Direct route for the connected subnet
        self.routing_table.push(Route {
            destination: subnet,
            next_hop: NextHop::Direct(idx),
            metric: 0,
        });

        idx
    }

    /// Add a static route.
    pub fn add_route(&mut self, destination: Subnet, next_hop: NextHop, metric: u8) {
        self.routing_table.push(Route {
            destination,
            next_hop,
            metric,
        });
        // Sort by metric (lowest first) for longest-prefix match tiebreaking
        self.routing_table.sort_by_key(|r| r.metric);
    }

    /// Add a firewall rule.
    pub fn add_firewall_rule(&mut self, rule: FirewallRule) {
        self.firewall.push(rule);
    }

    /// Check firewall rules for a packet. Returns true if the packet is allowed.
    fn check_firewall(&self, packet: &Packet) -> bool {
        for rule in &self.firewall {
            let src_match = rule
                .src
                .as_ref()
                .map_or(true, |s| s.contains(packet.src_ip));
            let dst_match = rule
                .dst
                .as_ref()
                .map_or(true, |s| s.contains(packet.dst_ip));
            let port_match = rule.dst_port.map_or(true, |p| p == packet.dst_port);

            if src_match && dst_match && port_match {
                return rule.action == FirewallAction::Accept;
            }
        }

        // Default policy: accept (no rules matched)
        true
    }

    /// Lookup the routing table for the best route to a destination.
    fn lookup_route(&self, dst: Ipv4Addr) -> Option<&Route> {
        // Find the most specific (longest prefix) matching route
        self.routing_table
            .iter()
            .filter(|r| r.destination.contains(dst))
            .max_by_key(|r| r.destination.prefix())
    }

    /// Route a packet — the main entry point.
    ///
    /// 1. Check firewall
    /// 2. Apply DNAT (if applicable)
    /// 3. Lookup route
    /// 4. Apply SNAT (if applicable)
    /// 5. Decrement TTL
    /// 6. Return the routing decision
    pub fn route_packet(&mut self, mut packet: Packet) -> RouteResult {
        // 1. Firewall check
        if !self.check_firewall(&packet) {
            return RouteResult::Drop(DropReason::Firewall);
        }

        // 2. DNAT — rewrite destination if a port-forwarding rule matches
        match self.nat_table.apply_dnat(&packet) {
            NatAction::Translated(new_dst_ip, new_dst_port) => {
                packet.dst_ip = new_dst_ip;
                packet.dst_port = new_dst_port;
            }
            NatAction::NoMatch => {}
        }

        // 3. Route lookup
        let route = match self.lookup_route(packet.dst_ip) {
            Some(r) => r.clone(),
            None => return RouteResult::Drop(DropReason::NoRoute),
        };

        // 4. SNAT — rewrite source if outbound rule matches
        match self.nat_table.apply_snat(&packet) {
            NatAction::Translated(new_src_ip, new_src_port) => {
                self.nat_table.track_connection(&packet, new_src_ip);
                packet.src_ip = new_src_ip;
                if new_src_port != 0 {
                    packet.src_port = new_src_port;
                }
            }
            NatAction::NoMatch => {}
        }

        // 5. TTL
        if !packet.decrement_ttl() {
            return RouteResult::Drop(DropReason::TtlExpired);
        }

        // 6. Forward based on route type
        match route.next_hop {
            NextHop::Direct(idx) => RouteResult::Deliver {
                interface_idx: idx,
                packet,
            },
            NextHop::Gateway(ip) => RouteResult::Forward {
                next_hop_ip: ip,
                packet,
            },
            NextHop::NetManager => RouteResult::CrossPod(packet),
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_router() -> Router {
        let mut router = Router::new();

        // Connect to two subnets
        let subnet1 = Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 24);
        let subnet2 = Subnet::new(Ipv4Addr::new(10, 0, 2, 0), 24);

        router.add_interface(Ipv4Addr::new(10, 0, 1, 1), subnet1);
        router.add_interface(Ipv4Addr::new(10, 0, 2, 1), subnet2);

        router
    }

    #[test]
    fn test_direct_route() {
        let mut router = setup_router();

        let pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 2),
            1234,
            Ipv4Addr::new(10, 0, 2, 3),
            80,
            vec![],
        );

        match router.route_packet(pkt) {
            RouteResult::Deliver { interface_idx, packet } => {
                assert_eq!(interface_idx, 1); // subnet2 interface
                assert_eq!(packet.dst_ip, Ipv4Addr::new(10, 0, 2, 3));
            }
            other => panic!("Expected Deliver, got something else"),
        }
    }

    #[test]
    fn test_no_route() {
        let mut router = setup_router();

        let pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 2),
            1234,
            Ipv4Addr::new(192, 168, 1, 1), // Not in any connected subnet
            80,
            vec![],
        );

        match router.route_packet(pkt) {
            RouteResult::Drop(DropReason::NoRoute) => {} // Expected
            other => panic!("Expected NoRoute drop"),
        }
    }

    #[test]
    fn test_cross_pod_route() {
        let mut router = setup_router();

        // Add a route for 10.0.3.0/24 via NetManager
        router.add_route(
            Subnet::new(Ipv4Addr::new(10, 0, 3, 0), 24),
            NextHop::NetManager,
            10,
        );

        let pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 2),
            1234,
            Ipv4Addr::new(10, 0, 3, 5),
            80,
            vec![],
        );

        match router.route_packet(pkt) {
            RouteResult::CrossPod(packet) => {
                assert_eq!(packet.dst_ip, Ipv4Addr::new(10, 0, 3, 5));
            }
            other => panic!("Expected CrossPod"),
        }
    }

    #[test]
    fn test_firewall_drop() {
        let mut router = setup_router();

        // Block all traffic to port 22
        router.add_firewall_rule(FirewallRule {
            src: None,
            dst: None,
            dst_port: Some(22),
            action: FirewallAction::Drop,
        });

        let pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 2),
            1234,
            Ipv4Addr::new(10, 0, 2, 3),
            22,
            vec![],
        );

        match router.route_packet(pkt) {
            RouteResult::Drop(DropReason::Firewall) => {} // Expected
            other => panic!("Expected Firewall drop"),
        }
    }

    #[test]
    fn test_ttl_expired() {
        let mut router = setup_router();

        let mut pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 2),
            1234,
            Ipv4Addr::new(10, 0, 2, 3),
            80,
            vec![],
        );
        pkt.ttl = 1; // Will become 0 after decrement

        match router.route_packet(pkt) {
            RouteResult::Drop(DropReason::TtlExpired) => {} // Expected
            other => panic!("Expected TtlExpired drop"),
        }
    }

    #[test]
    fn test_gateway_route() {
        let mut router = setup_router();

        // Add a route to 10.0.5.0/24 via gateway 10.0.2.100
        router.add_route(
            Subnet::new(Ipv4Addr::new(10, 0, 5, 0), 24),
            NextHop::Gateway(Ipv4Addr::new(10, 0, 2, 100)),
            5,
        );

        let pkt = Packet::tcp(
            Ipv4Addr::new(10, 0, 1, 2),
            1234,
            Ipv4Addr::new(10, 0, 5, 10),
            443,
            vec![],
        );

        match router.route_packet(pkt) {
            RouteResult::Forward { next_hop_ip, .. } => {
                assert_eq!(next_hop_ip, Ipv4Addr::new(10, 0, 2, 100));
            }
            other => panic!("Expected Forward"),
        }
    }
}
