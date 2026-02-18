# Nulltrace Core — Documentation

Welcome to the **nulltrace-core** documentation, the virtualization engine that simulates virtual machines (VMs) with scriptable processes powered by Luau.

## General Architecture

The core is composed of three main layers:

```
┌─────────────────────────────────────┐
│      VmManager (vm_manager.rs)      │   ← Central orchestrator
│  DB · DNS · NetManager · Router     │
├─────────────────────────────────────┤
│     Game Loop (run_loop @ 60 TPS)   │   ← Tick all VMs + network routing
├─────────────────────────────────────┤
│        VirtualMachine (vm.rs)       │   ← Isolated container with UUID + NIC
├─────────────────────────────────────┤
│             OS (os.rs)              │   ← Process scheduler + Luau sandbox
├─────────────────────────────────────┤
│          Process (process.rs)       │   ← Luau thread with lifecycle
└─────────────────────────────────────┘
```

## Index

| Document | Description |
|---|---|
| [VirtualMachine](./vm.md) | VM structure, creation and identification |
| [OS & Scheduler](./os.md) | Simulated operating system, Luau sandbox and scheduling |
| [Process](./process.md) | Process lifecycle, Luau threads and states |
| [VM Manager](./vm-manager.md) | Central orchestrator: VM lifecycle, networking, game loop |
| [Game Loop & Stress Test](./game-loop.md) | Main loop, frame timing and benchmark |
| [Server & gRPC](./server.md) | gRPC server and communication protocol |
| [Networking](./networking.md) | Full network simulation: IP, subnets, packets, NICs, routers, NAT, DNS, cross-pod |
| [HTTP Protocol](./http-protocol.md) | HTTP-like protocol for VM-to-VM communication (request/response, Lua API) |
| [Lua examples](./lua-examples.md) | Practical Lua script examples: I/O, fs, os, net (client and server) |
| [Blob Store](./blob-store.md) | Content-addressable storage: deduplication and copy-on-write for file content |
| [Nexus HTTP Server & Bootstrap](./nexus-http-server-and-bootstrap.md) | Nexus VM, httpd bin program, and /etc/bootstrap startup pattern |

## Testing

DB-backed tests (cluster binary) must run with a single thread to avoid migration deadlocks and ensure isolation:

```bash
cargo test --bin cluster -- --test-threads=1
```

PostgreSQL must be running at `postgres://nulltrace:nulltrace@localhost:5432/nulltrace`.

## Tech Stack

| Crate | Version | Usage |
|---|---|---|
| `tokio` | 1.x | Async runtime, sleep and timing |
| `mlua` (Luau) | 0.11.1 | Sandboxed scripting engine |
| `uuid` | 1.x | Unique IDs per VM |
| `dashmap` | 5.x | Concurrent HashMap |
| `tonic` / `prost` | 0.9 / 0.11 | gRPC server and protobuf |
| `redis` | 0.23 | Inter-node cluster communication |
| `sysinfo` | 0.29 | System metrics |
