# Networking System

> Files: `src/cluster/net/`

The networking system simulates **real-world computer networks** inside nulltrace. VMs can send packets to each other, routers forward traffic between subnets, firewalls block unwanted connections, and NAT hides internal IPs behind public ones — just like the real internet works.

This document explains every component from the ground up. No prior networking knowledge is assumed.

---

## Table of Contents

1. [The Big Picture](#the-big-picture)
2. [IP Addresses (`ip.rs`)](#ip-addresses)
3. [Subnets (`ip.rs`)](#subnets)
4. [Packets (`packet.rs`)](#packets)
5. [NIC — Network Interface Card (`nic.rs`)](#nic--network-interface-card)
6. [Router (`router.rs`)](#router)
7. [Firewall (`router.rs`)](#firewall)
8. [NAT — Network Address Translation (`nat.rs`)](#nat--network-address-translation)
9. [DNS — Domain Name System (`dns.rs`)](#dns--domain-name-system)
10. [NetManager — Cross-Pod Communication (`net_manager.rs`)](#netmanager--cross-pod-communication)
11. [How It All Connects](#how-it-all-connects)

---

## The Big Picture

Think of the networking system like a miniature version of the real internet:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    VM 1      │     │    VM 2      │     │    VM 3      │
│  10.0.1.2    │     │  10.0.1.3    │     │  10.0.2.2    │
│  (has a NIC) │     │  (has a NIC) │     │  (has a NIC) │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
  ─────┴────────────────────┴────           ─────┴─────────
       Subnet 10.0.1.0/24                  Subnet 10.0.2.0/24
              │                                   │
         ┌────┴───────────────────────────────────┴────┐
         │              Router (10.0.0.1)              │
         │   Routing Table + Firewall + NAT            │
         └─────────────────────────────────────────────┘
```

**In plain English:**
- Each **VM** is like a computer. It has an IP address and a network card (NIC).
- VMs in the same **subnet** can talk directly (like computers plugged into the same switch).
- To talk to a VM on a **different subnet**, packets go through a **router**.
- The router has **firewall rules** (to block traffic) and **NAT** (to hide internal IPs).

---

## IP Addresses

> File: `src/cluster/net/ip.rs` — struct `Ipv4Addr`

### What is an IP address?

An IP address is like a **home address for a computer**. Just like mail needs your street address to be delivered, network packets need an IP address to find the right machine.

An IPv4 address is 4 numbers separated by dots, each between 0 and 255:

```
10.0.1.2
│  │ │ │
│  │ │ └── Host number (which specific machine)
│  │ └──── Subnet segment
│  └────── Network segment
└───────── Top-level network
```

### How it works in code

```rust
// Create an IP address
let ip = Ipv4Addr::new(10, 0, 1, 2);

// Parse from a string
let ip = Ipv4Addr::parse("10.0.1.2").unwrap();

// Display it
println!("{}", ip);  // Prints: 10.0.1.2

// Compare two IPs
let ip_a = Ipv4Addr::new(10, 0, 1, 2);
let ip_b = Ipv4Addr::new(10, 0, 1, 3);
assert!(ip_a != ip_b);  // Different machines
```

### Internal representation

Under the hood, an IP is just 4 bytes stored in an array:

```rust
pub struct Ipv4Addr {
    octets: [u8; 4],   // [10, 0, 1, 2]
}
```

For math operations (like checking if an IP is inside a subnet), we convert it to a single 32-bit number:

```
10.0.1.2   →   0x0A000102   →   167772418 (decimal)
                 ││││││││
                 10 00 01 02  (each octet becomes 2 hex digits)
```

```rust
let ip = Ipv4Addr::new(10, 0, 1, 2);
let as_number = ip.to_u32();         // 167772418
let back = Ipv4Addr::from_u32(as_number);  // 10.0.1.2
```

### Private IP ranges

In the real world (and in our game), certain IP ranges are "private" — they're only used inside local networks, not on the public internet. Our game uses these:

| Range | Mask | Example |
|---|---|---|
| `10.0.0.0/8` | 10.x.x.x | `10.0.1.2` — most common in our game |
| `172.16.0.0/12` | 172.16.x.x – 172.31.x.x | `172.16.5.1` |
| `192.168.0.0/16` | 192.168.x.x | `192.168.1.100` |

```rust
let ip = Ipv4Addr::new(10, 0, 1, 2);
assert!(ip.is_private());   // true — it's in 10.0.0.0/8

let ip = Ipv4Addr::new(8, 8, 8, 8);
assert!(!ip.is_private());  // false — that's Google's public DNS
```

---

## Subnets

> File: `src/cluster/net/ip.rs` — struct `Subnet`

### What is a subnet?

A subnet is a **group of IP addresses** that belong to the same local network. Think of it like a neighborhood — all houses on the same street share the same zip code.

A subnet is written in **CIDR notation**: `10.0.1.0/24`

```
10.0.1.0/24
│        │
│        └── Prefix: the first 24 bits are the "network part"
└─────────── Network address: the starting IP
```

The `/24` means: "the first 24 bits (3 octets) identify the network, the remaining 8 bits (1 octet) identify the host."

This gives us:
- **Network address**: `10.0.1.0` (the subnet itself)
- **Usable IPs**: `10.0.1.1` through `10.0.1.254` (254 hosts)
- **Broadcast**: `10.0.1.255` (message to ALL hosts in the subnet)

### Visual explanation

```
Subnet: 10.0.1.0/24

         Network part (fixed)    Host part (varies)
         ┌──────────────────┐   ┌──────┐
Address: 10  .  0  .  1     .   ???
         └──────────────────┘   └──────┘
         These 3 octets are     This octet
         the same for all       identifies each
         hosts in the subnet    individual machine

10.0.1.0   = Network address (reserved, cannot be assigned)
10.0.1.1   = Gateway (the router's address in this subnet)
10.0.1.2   = First usable host address
10.0.1.3   = Second usable host address
  ...
10.0.1.254 = Last usable host address
10.0.1.255 = Broadcast address (reserved, sends to everyone)
```

### How it works in code

```rust
// Create a subnet
let subnet = Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 24);

// Parse from CIDR string
let subnet = Subnet::parse("10.0.1.0/24").unwrap();

// Key addresses
subnet.network();    // 10.0.1.0   — the network itself
subnet.gateway();    // 10.0.1.1   — the router
subnet.broadcast();  // 10.0.1.255 — the broadcast
subnet.mask();       // 255.255.255.0
subnet.host_count(); // 254 usable addresses
```

### Containment check

The most important operation: "does this IP belong to this subnet?"

```rust
let subnet = Subnet::parse("10.0.1.0/24").unwrap();

subnet.contains(Ipv4Addr::new(10, 0, 1, 50));   // true  ✓ same network
subnet.contains(Ipv4Addr::new(10, 0, 1, 200));  // true  ✓ same network
subnet.contains(Ipv4Addr::new(10, 0, 2, 1));    // false ✗ different subnet
subnet.contains(Ipv4Addr::new(192, 168, 1, 1)); // false ✗ completely different
```

**How it works internally**: apply the subnet mask to both addresses and compare:

```
IP:      10.0.1.50      →  00001010.00000000.00000001.00110010
Mask:    255.255.255.0   →  11111111.11111111.11111111.00000000
Result:  10.0.1.0        →  00001010.00000000.00000001.00000000  ← network part

Network: 10.0.1.0        →  00001010.00000000.00000001.00000000  ← matches!
```

### Auto-allocation

Subnets can automatically assign the next available IP:

```rust
let mut subnet = Subnet::parse("10.0.1.0/24").unwrap();

let ip1 = subnet.allocate_next();  // Some(10.0.1.2)  — first host
let ip2 = subnet.allocate_next();  // Some(10.0.1.3)  — second host
let ip3 = subnet.allocate_next();  // Some(10.0.1.4)  — third host
// ...continues until 10.0.1.254, then returns None
```

Note: `.1` is reserved for the gateway (router), so allocation starts at `.2`.

---

## Packets

> File: `src/cluster/net/packet.rs` — struct `Packet`

### What is a packet?

A packet is a **message sent across the network**. When VM 1 wants to talk to VM 2, it sends a packet — like putting a letter in an envelope.

The packet has:
- **Source IP + Port**: who is sending (the return address)
- **Destination IP + Port**: who should receive it
- **Protocol**: how the data should be interpreted (TCP, UDP, ICMP)
- **TTL**: how many routers it can pass through before being dropped
- **Payload**: the actual data being sent

```
┌─────────────────────────────────────────────────┐
│                    PACKET                       │
├─────────────────────┬───────────────────────────┤
│       HEADER        │         PAYLOAD           │
├─────────────────────┤                           │
│ src: 10.0.1.2:12345 │  "GET / HTTP/1.1\r\n"    │
│ dst: 10.0.2.3:80    │  "Host: webserver\r\n"    │
│ protocol: TCP       │  "\r\n"                   │
│ ttl: 64             │                           │
└─────────────────────┴───────────────────────────┘
```

### Protocols explained

| Protocol | What it's for | Analogy |
|---|---|---|
| **TCP** | Reliable data transfer (web, SSH, etc.) | Certified mail — guaranteed delivery |
| **UDP** | Fast, no guarantees (DNS, gaming) | Regular mail — fast but might get lost |
| **ICMP** | Network diagnostics (ping) | Shouting "are you there?" |

### Creating packets

```rust
// TCP packet — like a web request
let web_request = Packet::tcp(
    Ipv4Addr::new(10, 0, 1, 2),   // from this VM
    12345,                          // from this port (random high port)
    Ipv4Addr::new(10, 0, 2, 3),   // to this VM
    80,                             // to port 80 (HTTP)
    b"GET / HTTP/1.1\r\n".to_vec(), // the actual data
);

// UDP packet — like a DNS query
let dns_query = Packet::udp(
    Ipv4Addr::new(10, 0, 1, 2),
    54321,
    Ipv4Addr::new(10, 0, 0, 53),  // DNS server
    53,                             // DNS port
    b"query: example.com".to_vec(),
);

// ICMP ping — "are you alive?"
let ping = Packet::icmp_echo(
    Ipv4Addr::new(10, 0, 1, 2),   // from
    Ipv4Addr::new(10, 0, 1, 3),   // to
);

// The response to a ping
let pong = Packet::icmp_reply(&ping);
// pong.src = 10.0.1.3, pong.dst = 10.0.1.2 (reversed)
```

### TTL (Time To Live)

TTL prevents packets from looping forever. Every time a packet passes through a router, TTL decreases by 1. When it reaches 0, the packet is **dropped**.

```
Original packet:  TTL = 64
After router 1:   TTL = 63
After router 2:   TTL = 62
...
After router 64:  TTL = 0  → DROPPED! (no infinite loops)
```

```rust
let mut packet = Packet::icmp_echo(src, dst);
assert_eq!(packet.ttl, 64);

packet.decrement_ttl();  // returns true  (TTL = 63, still alive)
packet.decrement_ttl();  // returns true  (TTL = 62)

packet.ttl = 1;
packet.decrement_ttl();  // returns false (TTL = 0, DROP IT!)
```

### Packet size

Each packet has a simulated size (header + payload), useful for bandwidth calculations:

```rust
let pkt = Packet::tcp(src, 1234, dst, 80, vec![0u8; 100]);
pkt.size();  // 140 bytes  (40 byte TCP/IP header + 100 byte payload)

let pkt = Packet::udp(src, 1234, dst, 53, vec![0u8; 50]);
pkt.size();  // 78 bytes   (28 byte UDP/IP header + 50 byte payload)
```

---

## NIC — Network Interface Card

> File: `src/cluster/net/nic.rs` — struct `NIC`

### What is a NIC?

A NIC is the **network card** inside a computer. In the real world, it's the hardware that plugs into the Ethernet cable or connects to WiFi. In our simulation, every VM gets one NIC.

The NIC is responsible for:
- **Sending packets** (outbound buffer)
- **Receiving packets** (inbound buffer)
- **Knowing its own IP address** and which subnet it's on
- **Listening on ports** (like a web server listening on port 80)

```
┌──────────────── VM ─────────────────┐
│                                      │
│   Lua Process                        │
│     │                                │
│     │ net.send("10.0.2.3", 80, ...) │
│     ▼                                │
│   ┌─────────── NIC ───────────────┐ │
│   │ IP: 10.0.1.2                  │ │
│   │ MAC: 02:00:0a:00:01:02       │ │
│   │ Gateway: 10.0.1.1            │ │
│   │                               │ │
│   │ Outbound: [pkt1, pkt2, ...]  │──┼──→ To Router
│   │ Inbound:  [pkt3, ...]        │←─┼── From Router
│   │                               │ │
│   │ Listening: [80, 443, 22]     │ │
│   └───────────────────────────────┘ │
└──────────────────────────────────────┘
```

### Creating a NIC

```rust
// Option 1: Manually assign an IP
let nic = NIC::new(
    Ipv4Addr::new(10, 0, 1, 2),
    Subnet::parse("10.0.1.0/24").unwrap(),
);

// Option 2: Auto-allocate from the subnet
let mut subnet = Subnet::parse("10.0.1.0/24").unwrap();
let nic = NIC::from_subnet(&mut subnet).unwrap();
// nic.ip = 10.0.1.2 (auto-assigned)
```

### Sending and receiving

```rust
let mut nic = NIC::new(ip, subnet);

// === SENDING ===
// A Lua script calls net.send() → OS puts a packet in the outbound buffer
nic.send(Packet::tcp(my_ip, 12345, target_ip, 80, data));

// Each tick, the router drains outbound packets
let packets_to_route = nic.drain_outbound();  // Vec<Packet>

// === RECEIVING ===
// The router delivers a packet to this NIC
nic.deliver(incoming_packet);

// The Lua script reads it
if nic.has_inbound() {
    let packet = nic.recv().unwrap();
    println!("Got data from {}: {:?}", packet.src_ip, packet.payload);
}
```

### Port listening

A NIC can "listen" on ports — this is how servers work. If a VM is running a web server, it listens on port 80:

```rust
nic.listen(80);     // Start accepting connections on port 80
nic.listen(443);    // Also listen on HTTPS

nic.is_listening(80);   // true
nic.is_listening(22);   // false — not listening on SSH

nic.unlisten(80);       // Stop listening
```

### MAC address

Each NIC has a MAC address (hardware address). In our simulation, it's generated deterministically from the IP:

```rust
let nic = NIC::new(Ipv4Addr::new(10, 0, 1, 2), subnet);
nic.mac_string();  // "02:00:0a:00:01:02"
//                     │     │  │  │  │
//                     │     10. 0. 1. 2  (IP octets in hex)
//                     └── 02:00 prefix (locally administered)
```

---

## Router

> File: `src/cluster/net/router.rs` — struct `Router`

### What is a router?

A router is a device that **connects different subnets** and forwards packets between them. Without a router, VMs in subnet `10.0.1.0/24` can't talk to VMs in subnet `10.0.2.0/24`.

Think of it like a post office: letters come in, the post office checks the destination address, and sends it out the correct door.

```
              ┌──────────────────────────────────────┐
              │              ROUTER                  │
              │                                      │
     ─────┬──┤ Interface 0:  10.0.1.1 (subnet /24)  │
  Subnet  │  │                                      │
  10.0.1  │  │ Interface 1:  10.0.2.1 (subnet /24)  ├──┬─────
          │  │                                      │  │  Subnet
          │  │ Routing Table:                       │  │  10.0.2
          │  │   10.0.1.0/24 → Interface 0 (direct) │  │
          │  │   10.0.2.0/24 → Interface 1 (direct) │  │
          │  │   10.0.3.0/24 → NetManager (cross-pod)│ │
          │  │                                      │  │
          │  │ Firewall: [rules...]                 │  │
          │  │ NAT Table: [rules...]                │  │
              └──────────────────────────────────────┘
```

### Setting up a router

```rust
let mut router = Router::new();

// Connect the router to two subnets
// This automatically adds a "Direct" route for each subnet
router.add_interface(
    Ipv4Addr::new(10, 0, 1, 1),               // Router's IP on this subnet
    Subnet::parse("10.0.1.0/24").unwrap(),     // The subnet
);
router.add_interface(
    Ipv4Addr::new(10, 0, 2, 1),
    Subnet::parse("10.0.2.0/24").unwrap(),
);

// Now the router can forward packets between 10.0.1.x and 10.0.2.x!
```

### The routing table

The routing table is the router's "map" — it tells the router where to send each packet based on the destination IP.

Each entry says: "if the destination is in **this subnet**, send it **this way**."

```rust
// The routing table (auto-populated when adding interfaces):
//
// Destination       Next Hop           Metric
// ─────────────────────────────────────────────
// 10.0.1.0/24  →   Direct(0)           0     ← Interface 0 is connected
// 10.0.2.0/24  →   Direct(1)           0     ← Interface 1 is connected

// Add a static route for a remote subnet
router.add_route(
    Subnet::parse("10.0.3.0/24").unwrap(),
    NextHop::Gateway(Ipv4Addr::new(10, 0, 2, 100)),  // Forward to another router
    5,
);

// Add a cross-pod route (traffic goes through Redis)
router.add_route(
    Subnet::parse("10.0.5.0/24").unwrap(),
    NextHop::NetManager,  // Send via NetManager → Redis → other pod
    10,
);
```

### Three types of next hops

| Type | When | Example |
|---|---|---|
| `Direct(idx)` | Destination is on a directly connected subnet | VM at 10.0.1.5 is on interface 0 |
| `Gateway(ip)` | Destination is reachable through another router | Forward to router at 10.0.2.100 |
| `NetManager` | Destination is on a different K8s pod | Send over Redis Pub/Sub |

### Routing a packet: step by step

When a packet enters the router, here's what happens:

```rust
let result = router.route_packet(packet);
```

```
Packet arrives: 10.0.1.2:12345 → 10.0.2.3:80

Step 1: FIREWALL CHECK
  → Is this packet allowed? Check rules in order.
  → If any rule says DROP → packet is discarded.
  → If no rule matches → default ACCEPT.

Step 2: DNAT (Destination NAT)
  → Should we rewrite the destination?
  → Example: public IP 203.0.113.1:80 → internal 10.0.1.5:8080
  → If no DNAT rule matches → skip.

Step 3: ROUTE LOOKUP
  → Where does 10.0.2.3 belong?
  → Check routing table: 10.0.2.0/24 → Direct(1) ✓
  → Uses longest-prefix match (most specific subnet wins).

Step 4: SNAT (Source NAT)
  → Should we rewrite the source?
  → Example: 10.0.1.2 → 203.0.113.1 (hide internal IP)
  → If yes → track the connection for return traffic.

Step 5: TTL DECREMENT
  → TTL = 64 → 63
  → If TTL reaches 0 → drop the packet.

Step 6: FORWARD
  → Send the packet out the correct interface.
```

### Route results

```rust
match router.route_packet(packet) {
    RouteResult::Deliver { interface_idx, packet } => {
        // Packet goes to a VM on the directly connected subnet
        // interface_idx tells you which subnet
    }
    RouteResult::Forward { next_hop_ip, packet } => {
        // Packet goes to another router
    }
    RouteResult::CrossPod(packet) => {
        // Packet goes to NetManager → Redis → another K8s pod
    }
    RouteResult::Drop(reason) => {
        // Packet was dropped (NoRoute, TtlExpired, or Firewall)
    }
}
```

### Example: Multi-hop routing

```
VM 1 (10.0.1.2) wants to reach VM 4 (10.0.5.10)

Packet: 10.0.1.2 → 10.0.5.10

Router A:
  Routing table lookup: 10.0.5.0/24 → Gateway(10.0.2.100)
  → Forward packet to router at 10.0.2.100
  → TTL: 64 → 63

Router B (10.0.2.100):
  Routing table lookup: 10.0.5.0/24 → Direct(1)
  → Deliver to VM 4 on interface 1
  → TTL: 63 → 62

VM 4 receives the packet with TTL=62
```

---

## Firewall

> File: `src/cluster/net/router.rs` — struct `FirewallRule`

### What is a firewall?

A firewall is a **gatekeeper** on the router. It checks every packet and decides: should this be allowed through, or should it be blocked?

Rules are evaluated **in order** — the first rule that matches wins.

### Firewall rules

Each rule can filter by:
- **Source subnet** (where the packet came from)
- **Destination subnet** (where it's going)
- **Destination port** (which service it's trying to reach)

```rust
// Block all SSH traffic (port 22) from any source
router.add_firewall_rule(FirewallRule {
    src: None,                    // Any source
    dst: None,                    // Any destination
    dst_port: Some(22),           // SSH port
    action: FirewallAction::Drop, // BLOCK IT
});

// Allow traffic from 10.0.1.0/24 to 10.0.2.0/24 on port 80
router.add_firewall_rule(FirewallRule {
    src: Some(Subnet::parse("10.0.1.0/24").unwrap()),
    dst: Some(Subnet::parse("10.0.2.0/24").unwrap()),
    dst_port: Some(80),
    action: FirewallAction::Accept,
});
```

### Example: What happens when a packet is checked

```
Packet: 10.0.1.2:54321 → 10.0.2.3:22 (SSH)

Rule 1: src=Any, dst=Any, port=22, action=DROP
  → Source matches? YES (any)
  → Dest matches?   YES (any)
  → Port matches?   YES (22 == 22)
  → FIRST MATCH → DROP ✗

The packet is discarded. VM 1 cannot SSH into VM 3.
```

```
Packet: 10.0.1.2:54321 → 10.0.2.3:80 (HTTP)

Rule 1: src=Any, dst=Any, port=22, action=DROP
  → Port matches? NO (80 ≠ 22) → skip this rule

Rule 2: src=10.0.1.0/24, dst=10.0.2.0/24, port=80, action=ACCEPT
  → Source matches? YES (10.0.1.2 ∈ 10.0.1.0/24)
  → Dest matches?   YES (10.0.2.3 ∈ 10.0.2.0/24)
  → Port matches?   YES (80 == 80)
  → FIRST MATCH → ACCEPT ✓

The packet is allowed through.
```

---

## NAT — Network Address Translation

> File: `src/cluster/net/nat.rs` — struct `NatTable`

### What is NAT?

NAT lets you **rewrite IP addresses** in packets as they pass through a router. There are two types:

| Type | Direction | What it does | Real-world example |
|---|---|---|---|
| **SNAT** | Outbound | Rewrites the **source** IP | Your home router replaces your PC's `192.168.1.5` with your public IP |
| **DNAT** | Inbound | Rewrites the **destination** IP | Port forwarding: traffic to your public IP port 80 goes to your internal web server |

### SNAT (Source NAT) — Hiding internal IPs

When a VM in a private subnet sends traffic to the "outside", SNAT replaces its internal IP with a public one.

```
BEFORE SNAT:
  Source: 10.0.1.5:12345  →  Destination: 8.8.8.8:53
         (internal IP)

AFTER SNAT:
  Source: 203.0.113.1:12345  →  Destination: 8.8.8.8:53
         (public IP)
```

```rust
// Setup: hide all IPs in 10.0.1.0/24 behind 203.0.113.1
nat_table.add_snat(
    Subnet::parse("10.0.1.0/24").unwrap(),
    Ipv4Addr::new(203, 0, 113, 1),
);

// When a packet from 10.0.1.5 leaves:
let pkt = Packet::tcp(
    Ipv4Addr::new(10, 0, 1, 5), 12345,  // internal source
    Ipv4Addr::new(8, 8, 8, 8), 53,      // external destination
    vec![],
);

match nat_table.apply_snat(&pkt) {
    NatAction::Translated(new_ip, _) => {
        // new_ip = 203.0.113.1
        // The packet's source is now rewritten
    }
    NatAction::NoMatch => { /* No SNAT rule matched */ }
}
```

### DNAT (Destination NAT) — Port forwarding

DNAT redirects incoming traffic to an internal machine. This is how you make an internal web server accessible from outside.

```
BEFORE DNAT:
  Source: 1.2.3.4:54321  →  Destination: 203.0.113.1:80
                                         (public IP, port 80)

AFTER DNAT:
  Source: 1.2.3.4:54321  →  Destination: 10.0.1.5:8080
                                         (internal server, port 8080)
```

```rust
// Setup: forward port 80 on the public IP to the internal web server
nat_table.add_dnat(
    Ipv4Addr::new(203, 0, 113, 1),  // external IP
    80,                               // external port
    Ipv4Addr::new(10, 0, 1, 5),     // internal IP
    8080,                             // internal port
);
```

### Connection tracking

When SNAT rewrites an outbound packet's source IP, the router needs to remember the original IP. Why? Because when the **reply** comes back, it's addressed to the public IP (`203.0.113.1`) — the router needs to know to forward it back to the original sender (`10.0.1.5`).

```
OUTBOUND (tracked):
  10.0.1.5:12345 → 8.8.8.8:53
  SNAT rewrites to: 203.0.113.1:12345 → 8.8.8.8:53
  Router remembers: "203.0.113.1:12345 was originally 10.0.1.5"

RETURN TRAFFIC (auto-resolved):
  8.8.8.8:53 → 203.0.113.1:12345
  Router checks connection table: "203.0.113.1:12345 = 10.0.1.5"
  Rewrites destination: 8.8.8.8:53 → 10.0.1.5:12345
  Delivered to VM!
```

```rust
// Track the original outbound connection
nat_table.track_connection(&original_packet, translated_src_ip);

// Later, when the reply comes back, DNAT lookup finds it automatically
match nat_table.apply_dnat(&reply_packet) {
    NatAction::Translated(original_ip, port) => {
        // original_ip = 10.0.1.5 — correctly mapped back!
    }
    _ => {}
}
```

---

## DNS — Domain Name System

> File: `src/cluster/net/dns.rs` — struct `DnsResolver`

### What is DNS?

DNS translates **human-readable names** into IP addresses. Instead of remembering `10.0.2.3`, you can use `corp-database.nulltrace.local`.

```
"corp-database.nulltrace.local"  →  DNS query  →  10.0.2.3
"web-server.nulltrace.local"     →  DNS query  →  10.0.1.5
```

### Record types

| Type | Purpose | Example |
|---|---|---|
| **A** | Name → IP | `web.corp` → `10.0.1.5` |
| **PTR** | IP → Name (reverse) | `10.0.1.5` → `web.corp` |
| **CNAME** | Alias → Real name | `www.corp` → `web.corp` |
| **MX** | Domain → Mail server | `corp` → `mail.corp` |

### Redis-backed storage

All DNS records are stored in Redis (shared across all K8s pods), so any VM on any pod can resolve any hostname:

```
Redis key                                  Value
─────────────────────────────────────────────────────────
dns:a:corp-database.nulltrace.local    →  "10.0.2.3"
dns:a:web-server.nulltrace.local       →  "10.0.1.5"
dns:ptr:10.0.2.3                       →  "corp-database.nulltrace.local"
dns:cname:www.nulltrace.local          →  "web-server.nulltrace.local"
```

> **Status**: DNS has a stub implementation. Redis integration will be added in Phase 3.

---

## NetManager — Cross-Pod Communication

> File: `src/cluster/net/net_manager.rs` — struct `NetManager`

### The problem

In production, nulltrace runs on **multiple K8s pods** (servers). VM 1 might be on Pod A, and VM 4 might be on Pod B. They can't talk directly — they're on different physical machines.

### The solution: Redis Pub/Sub

The NetManager acts as a **bridge** between pods. When a router can't deliver a packet locally, it hands it to the NetManager, which publishes it to Redis. The NetManager on the destination pod subscribes to its channel and receives the packet.

```
Pod A                         Redis                       Pod B
┌──────────┐                ┌───────┐                ┌──────────┐
│ VM 1     │                │       │                │ VM 4     │
│ 10.0.1.2 │   packet       │       │   packet       │ 10.0.3.2 │
│    │     │ ──────────→   │       │ ──────────→   │    ▲     │
│    ▼     │                │       │                │    │     │
│ Router   │                │       │                │ Router   │
│    │     │                │       │                │    ▲     │
│    ▼     │                │       │                │    │     │
│ NetMgr A │─── PUBLISH ──→│ Redis │─── SUBSCRIBE ─→│ NetMgr B │
│          │  net:pod:pod-b │ PubSub│  net:pod:pod-b │          │
└──────────┘                └───────┘                └──────────┘
```

### Redis channels and keys

| Channel/Key | Purpose |
|---|---|
| `net:pod:{cluster_id}` | Pub/Sub channel: incoming packets for a specific pod |
| `net:broadcast` | Pub/Sub channel: global announcements (new subnets, etc.) |
| `net:route:{subnet}` | String key: which pod owns this subnet |
| `net:arp:{ip}` | String key: which pod + VM owns this IP |
| `net:pods` | Set: all active pod IDs |

### How cross-pod delivery works

```
1. VM 1 (Pod A, 10.0.1.2) sends packet to VM 4 (Pod B, 10.0.3.2)

2. VM 1's NIC puts packet in outbound buffer

3. Router (Pod A) does a routing table lookup:
   - 10.0.3.0/24 → NextHop::NetManager  (not on any local interface)

4. Router returns RouteResult::CrossPod(packet)

5. NetManager (Pod A) looks up Redis:
   - GET net:route:10.0.3.0/24 → "pod-b"

6. NetManager (Pod A) publishes the packet:
   - PUBLISH net:pod:pod-b <serialized packet>

7. NetManager (Pod B) receives the packet (subscribed to net:pod:pod-b)

8. NetManager (Pod B) hands it to the local Router

9. Router (Pod B) delivers to VM 4's NIC inbound buffer

10. VM 4's Lua process reads the packet:
    net.recv() → { src = "10.0.1.2", data = "hello!" }
```

> **Status**: NetManager has a stub implementation. Redis Pub/Sub integration will be added in Phase 4.

---

## How It All Connects

Here's the complete flow of a packet from one VM to another, touching every component:

```
VM 1 runs Lua script:
  net.send("10.0.2.3", 80, "hello")
         │
         ▼
    ┌─── NIC (10.0.1.2) ───┐
    │ Creates Packet:        │
    │   src: 10.0.1.2:49152 │
    │   dst: 10.0.2.3:80    │
    │   protocol: TCP        │
    │   ttl: 64              │
    │   payload: "hello"     │
    │                        │
    │ → outbound buffer      │
    └────────┬───────────────┘
             │
             ▼ (router drains outbound each tick)
    ┌─── Router ─────────────────────────────────────────┐
    │                                                     │
    │  1. Firewall: no rules match → ACCEPT              │
    │                                                     │
    │  2. DNAT: no port-forwarding rule → skip           │
    │                                                     │
    │  3. Route lookup: 10.0.2.3 ∈ 10.0.2.0/24          │
    │     → Route: Direct(1) — interface 1               │
    │                                                     │
    │  4. SNAT: source 10.0.1.2 not in any SNAT rule    │
    │     → skip                                          │
    │                                                     │
    │  5. TTL: 64 → 63                                   │
    │                                                     │
    │  6. Result: Deliver { interface: 1, packet }       │
    └────────┬───────────────────────────────────────────┘
             │
             ▼ (deliver to VM on interface 1's subnet)
    ┌─── NIC (10.0.2.3) ───┐
    │ ← inbound buffer      │
    │                        │
    │ is_listening(80)? YES  │
    │ → deliver to process   │
    └────────┬───────────────┘
             │
             ▼
VM 3 Lua script receives:
  net.recv() → {
    src = "10.0.1.2",
    port = 49152,
    data = "hello"
  }
```

### Cross-pod variant

If VM 3 were on a different K8s pod, step 3 would return `NextHop::NetManager` instead of `Direct`, and the packet would travel through Redis Pub/Sub before arriving at the destination pod's router.

```
Same flow, but at step 3:
  Route lookup: 10.0.3.0/24 → NetManager
    │
    ▼
NetManager (Pod A)
    │  PUBLISH net:pod:pod-b <packet>
    │
    ▼ (via Redis)
NetManager (Pod B)
    │  Received packet from net:pod:pod-b
    │
    ▼
Router (Pod B)
    │  Route lookup: 10.0.3.2 ∈ 10.0.3.0/24 → Direct(0)
    │
    ▼
NIC (10.0.3.2) ← inbound buffer
```

---

## FAQ

### Can a single pod have multiple routers?

**Yes — fully supported.** The `NetManager` stores routers in a `Vec<Router>`, so each pod can have as many routers as needed:

```rust
pub struct NetManager {
    pub routers: Vec<Router>,  // ← No limit on the number of routers
    // ...
}

// Adding multiple routers to the same pod:
let mut net_manager = NetManager::new("pod-a".to_string());

let mut router_a = Router::new();
router_a.add_interface(Ipv4Addr::new(10, 0, 1, 1), Subnet::parse("10.0.1.0/24").unwrap());
router_a.add_interface(Ipv4Addr::new(10, 0, 2, 1), Subnet::parse("10.0.2.0/24").unwrap());

let mut router_b = Router::new();
router_b.add_interface(Ipv4Addr::new(10, 0, 3, 1), Subnet::parse("10.0.3.0/24").unwrap());
router_b.add_interface(Ipv4Addr::new(10, 0, 4, 1), Subnet::parse("10.0.4.0/24").unwrap());

net_manager.add_router(router_a);  // Connects subnets 1 & 2
net_manager.add_router(router_b);  // Connects subnets 3 & 4
```

This enables complex topologies on a single pod:

```
┌────────────────────────── Pod A ──────────────────────────┐
│                                                           │
│  ┌─ Router A ──────────────┐   ┌─ Router B ────────────┐ │
│  │ 10.0.1.0/24 ←→ 10.0.2.0│   │ 10.0.3.0/24 ←→ 10.0.4│ │
│  └────────┬────────────────┘   └────────┬───────────────┘ │
│           │                             │                 │
│      ┌────┴────┐  ┌────┐          ┌────┴────┐  ┌────┐   │
│      │ VM 1    │  │VM 2│          │ VM 3    │  │VM 4│   │
│      │10.0.1.2 │  │.1.3│          │10.0.3.2 │  │.4.2│   │
│      └─────────┘  └────┘          └─────────┘  └────┘   │
│                                                           │
│  Routers can also be interconnected via Gateway routes:   │
│  Router A: 10.0.3.0/24 → Gateway(10.0.2.100) → Router B │
│  Router B: 10.0.1.0/24 → Gateway(10.0.4.100) → Router A │
└───────────────────────────────────────────────────────────┘
```

Each router is fully independent — it has its own routing table, firewall rules, and NAT table. Routers on the same pod can forward traffic to each other using `NextHop::Gateway`, exactly like real-world routers chained together.

**In practice**, this means a single pod can simulate an entire corporate network with multiple departments (subnets), each behind its own router with its own firewall policies. The only limit is memory.
