# VM Manager

> File: `src/cluster/vm_manager.rs`

The `VmManager` is the **central orchestrator** for all virtual machines in a cluster pod. It owns the full VM lifecycle — creation, restoration, networking, execution, and destruction — coordinating between the database, filesystem, DNS, networking, and the game loop.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [VM Lifecycle](#vm-lifecycle)
4. [In-Memory State](#in-memory-state)
5. [Network Tick](#network-tick)
6. [The Game Loop](#the-game-loop)
7. [Startup Flow](#startup-flow)
8. [Relationships](#relationships)

---

## Overview

Before `VmManager` existed, VM creation, networking, DNS registration, and the game loop were loosely coupled. The manager centralizes all of this into a single struct that:

- **Creates VMs** — allocates IP from the subnet, inserts into PostgreSQL, bootstraps the filesystem, registers DNS and NetManager entries, and tracks the VM in memory.
- **Stops and destroys VMs** — cleanly unregisters from DNS/NetManager, updates DB status, and removes from the active list.
- **Restores VMs on startup** — queries the database for VMs that were `running` or `crashed`, re-registers them in DNS and NetManager, and returns records so the caller can rebuild `VirtualMachine` structs.
- **Runs the game loop** — ticks all VMs at 60 TPS, manages Lua context switching per-VM, and routes packets between VMs each tick.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                        VmManager                             │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────┐ │
│  │  VmService   │  │  FsService   │  │  DnsResolver         │ │
│  │  (PostgreSQL) │  │  (PostgreSQL) │  │  (in-memory A/PTR)   │ │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬──────────┘ │
│         │                 │                     │            │
│  ┌──────┴─────────────────┴─────────────────────┴──────────┐ │
│  │                    Orchestration Layer                   │ │
│  │   create_vm() · stop_vm() · destroy_vm() · restore_vms()│ │
│  └──────┬──────────────────────────────────────────────────┘ │
│         │                                                    │
│  ┌──────┴──────────────────────────────────────────────────┐ │
│  │                   Network Layer                          │ │
│  │  ┌──────────┐  ┌──────────┐  ┌────────────────────────┐ │ │
│  │  │  Subnet   │  │  Router   │  │  NetManager (Redis)    │ │ │
│  │  │  (IP pool) │  │  (routing │  │  (cross-pod, ARP,      │ │ │
│  │  │           │  │   + NAT   │  │   DNS cache)           │ │ │
│  │  │           │  │   + FW)   │  │                        │ │ │
│  │  └──────────┘  └──────────┘  └────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │                  active_vms: Vec<ActiveVm>                │ │
│  │  Lightweight in-memory records (id, hostname, ip)         │ │
│  └──────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

### Struct definition

```rust
pub struct VmManager {
    pub vm_service: Arc<VmService>,    // DB operations (CRUD, status)
    pub fs_service: Arc<FsService>,    // Filesystem bootstrap
    pub dns: DnsResolver,              // In-memory DNS (A, PTR, CNAME)
    pub net_manager: NetManager,       // Cross-pod networking via Redis
    pub subnet: Subnet,               // IP address pool for allocation
    pub router: Router,               // Packet routing + firewall + NAT

    active_vms: Vec<ActiveVm>,        // In-memory VM tracking
}
```

---

## VM Lifecycle

### Creating a VM

`create_vm(config)` performs **five steps atomically**:

```
VmConfig (hostname, cpu, memory, disk)
  │
  ▼
1. ALLOCATE IP
   Subnet::allocate_next() → NIC
   Fills config with ip, subnet, gateway, mac
  │
  ▼
2. INSERT INTO DATABASE
   VmService::create_vm() → VmRecord (status = "running")
  │
  ▼
3. BOOTSTRAP FILESYSTEM
   FsService::bootstrap_fs() → creates root dir tree in PostgreSQL
  │
  ▼
4. REGISTER IN DNS (conditional) + NETMANAGER
   if dns_name is set:
     DnsResolver::register_a(dns_name, ip)   → A + PTR records
   NetManager::register_vm(ip, uuid)         → local ARP + Redis ARP
  │
  ▼
5. TRACK IN MEMORY
   active_vms.push(ActiveVm { id, hostname, dns_name, ip })
  │
  ▼
Returns (VmRecord, NIC) → caller attaches NIC to VirtualMachine
```

```rust
let (record, nic) = manager.create_vm(VmConfig {
    hostname: "web-srv-01".to_string(),
    dns_name: Some("webserver.corp.local".to_string()), // Optional: only set for discoverable VMs
    cpu_cores: 2,
    memory_mb: 1024,
    disk_mb: 20480,
    ..Default::default()
}).await?;

let mut vm = VirtualMachine::with_id(&lua, record.id);
vm.attach_nic(nic);
```

### Stopping a VM

`stop_vm(vm_id)` gracefully shuts down a VM:

```
1. Set DB status to "stopped"
2. Remove from active_vms
3. Unregister DNS A/PTR records (only if dns_name was set)
4. Unregister from NetManager (local + Redis ARP)
```

The `VirtualMachine` struct and its processes still exist in memory until the game loop removes them (when all processes finish or the caller drops the struct).

### Destroying a VM

`destroy_vm(vm_id)` permanently deletes a VM:

```
1. Remove from active_vms + unregister DNS (if dns_name set) + NetManager
2. DELETE FROM vms (cascades to fs_nodes)
```

This is a hard delete — the VM's database record and entire filesystem are gone.

### Restoring VMs on startup

`restore_vms()` recovers VMs that were running when the process last shut down:

```
1. Query DB: SELECT * FROM vms WHERE status IN ('running', 'crashed')
2. For each record:
   a. Re-register in DNS (dns_name → IP) — only if dns_name is set
   b. Re-register in NetManager (IP → UUID)
   c. Track in active_vms
   d. Set status to "running"
3. Return Vec<VmRecord> → caller rebuilds VirtualMachine structs
```

This enables **crash recovery** — if the pod restarts, all previously-running VMs are restored to their last known state.

---

## In-Memory State

The `ActiveVm` struct is a lightweight record that avoids hitting the database on every tick:

```rust
pub struct ActiveVm {
    pub id: Uuid,
    pub hostname: String,
    pub dns_name: Option<String>,
    pub ip: Option<Ipv4Addr>,
}
```

The game loop uses this to set the `VmContext` before each VM's tick, providing the Lua scripts with the VM's identity (hostname, IP) without a database query.

**Why not just use VmRecord?** `VmRecord` includes fields like `cpu_cores`, `memory_mb`, `created_at` that are only needed for display/API purposes, not for the hot path. `ActiveVm` keeps the per-tick overhead minimal.

---

## Network Tick

`network_tick(vms)` runs once per game tick after all VMs have been ticked. It handles packet routing:

```
1. DRAIN OUTBOUND
   For each VM's NIC → collect all outbound packets

2. ROUTE EACH PACKET
   Router::route_packet(pkt) returns one of:
   │
   ├── Deliver { packet }  → Find destination VM by IP, deliver to NIC inbound
   ├── Forward { packet }  → Same as Deliver in single-router setup
   ├── CrossPod(packet)    → NetManager::send_cross_pod() via Redis
   └── Drop(reason)        → Packet discarded (no route, TTL, firewall)
```

```
Tick N:
  ┌─────────┐   ┌─────────┐   ┌─────────┐
  │  VM 1   │   │  VM 2   │   │  VM 3   │
  │ outbound│   │ outbound│   │ outbound│
  │ [pkt_a] │   │ [pkt_b] │   │  []     │
  └────┬────┘   └────┬────┘   └─────────┘
       │              │
       ▼              ▼
  ┌──────────────────────────────────┐
  │          Router                  │
  │  route(pkt_a) → Deliver to VM 3 │
  │  route(pkt_b) → CrossPod        │
  └──────────┬───────────┬──────────┘
             │           │
             ▼           ▼
        VM 3 NIC    NetManager
        inbound     → Redis
        [pkt_a]     → other pod
```

---

## The Game Loop

`run_loop(lua, vms)` is the main execution loop running at **60 TPS** (ticks per second):

```
┌─────────────────── LOOP (60 TPS) ──────────────────────┐
│                                                         │
│  For each VM:                                           │
│    1. SET CONTEXT                                       │
│       VmContext.set_vm(id, hostname, ip)                │
│       Load inbound packets from NIC → VmContext buffer  │
│                                                         │
│    2. TICK PROCESSES                                    │
│       For each process in vm.os.processes:              │
│         Set VmContext.current_pid                       │
│         process.tick() → resumes Lua coroutine          │
│       Remove finished processes                         │
│                                                         │
│    3. SYNC OUTBOUND                                     │
│       VmContext.net_outbound → NIC outbound buffer      │
│       VmContext.listening_ports → NIC.listen()          │
│                                                         │
│  NETWORK TICK                                           │
│    Route all outbound packets between VMs               │
│                                                         │
│  CLEANUP                                                │
│    Remove VMs where os.is_finished() == true            │
│                                                         │
│  FRAME PACING                                           │
│    Sleep until next tick (~16.6ms interval)             │
│                                                         │
│  LOG (every 5 seconds)                                  │
│    "[cluster] Tick N | X VMs active | uptime Ys"       │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Context switching

All VMs share a **single Lua state** (for memory efficiency — enables 400k+ VMs). The `VmContext` stored in `lua.app_data()` is swapped before each VM's tick so that Lua API calls (`fs.read()`, `net.send()`, `os.hostname()`) operate on the correct VM's data.

```rust
// Before ticking VM "web-srv-01" (10.0.1.5):
ctx.set_vm(vm.id, "web-srv-01", Some(Ipv4Addr(10,0,1,5)));

// Now when Lua calls os.hostname() → returns "web-srv-01"
// When Lua calls net.send()       → packet goes to this VM's NIC
// When Lua calls fs.read("/etc")  → reads this VM's filesystem
```

---

## Startup Flow

The full startup sequence in `main.rs`:

```
1. CONNECT TO POSTGRESQL
   db::connect() → PgPool
   db::run_migrations()

2. CREATE SERVICES
   VmService::new(pool)    → Arc<VmService>
   FsService::new(pool)    → Arc<FsService>

3. CREATE LUA STATE
   create_lua_state()
   lua.set_app_data(VmContext::new(pool))
   lua_api::register_all()  → fs, net, os APIs

4. CREATE VM MANAGER
   VmManager::new(vm_service, fs_service, Subnet(10.0.1.0/24))
   └── Creates Router with gateway interface
   └── Creates DnsResolver (empty)
   └── Creates NetManager("local")

5. CONNECT REDIS (optional)
   net_manager.connect_redis("redis://127.0.0.1:6379")
   └── If unavailable → runs without cross-pod support

6. RESTORE VMS FROM DATABASE
   manager.restore_vms()
   └── Queries running/crashed VMs
   └── Re-registers DNS + NetManager
   └── Returns Vec<VmRecord>

7. REBUILD VM STRUCTS
   For each record:
     VirtualMachine::with_id(lua, record.id)
     Parse IP/subnet from record → NIC::new() → vm.attach_nic()

8. START GAME LOOP
   manager.run_loop(lua, vms).await
   └── Runs forever at 60 TPS
```

---

## Relationships

| Component | Role | Owned by VmManager? |
|---|---|---|
| `VmService` | PostgreSQL CRUD for VM records | Shared (`Arc`) |
| `FsService` | PostgreSQL filesystem operations | Shared (`Arc`) |
| `DnsResolver` | In-memory DNS resolution (A, PTR, CNAME) | Yes |
| `NetManager` | Cross-pod networking, Redis ARP/DNS cache | Yes |
| `Subnet` | IP address pool for allocation | Yes |
| `Router` | Packet routing, firewall, NAT | Yes |
| `VirtualMachine` | The actual VM with OS and processes | No — owned by the caller, passed to `run_loop()` |
| `VmContext` | Per-tick Lua context (stored in Lua app_data) | No — owned by Lua state |

### Data flow summary

```
                    VmManager
                   ╱    │     ╲
                  ╱     │      ╲
            create    tick    network
               │       │        │
               ▼       ▼        ▼
          PostgreSQL   Lua    Router
          (VmService)  (ctx)  (packets)
          (FsService)    │       │
               │         │       ├── Deliver → NIC inbound
               │         │       ├── CrossPod → Redis
               │         │       └── Drop
               │         ▼
               │    VirtualMachine
               │      └── OS
               │           └── Process (Lua coroutine)
               ▼
            DNS + NetManager
            (registration)
```
