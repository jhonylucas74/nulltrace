# VirtualMachine

> File: `src/cluster/vm.rs`

The `VirtualMachine` is the fundamental unit of nulltrace-core. Each VM represents a **simulated computer** within the game, with its own operating system and processes.

## Structure

```rust
pub struct VirtualMachine<'a> {
    pub id: Uuid,       // Unique identifier (UUID v4)
    pub os: OS<'a>,     // The VM's operating system
    lua: &'a Lua,       // Reference to the shared Lua state
}
```

## How It Works

### Creation

```rust
let lua = create_lua_state();
let vm = VirtualMachine::new(&lua);
```

When creating a VM:
1. A unique **UUID v4** is automatically generated
2. An `OS` instance is created, bound to the Lua state
3. The VM receives a **shared reference** to `Lua` — all VMs use the **same** Luau runtime

### Why Share the Lua Runtime?

The `Lua` state is memory-heavy. By sharing a single runtime across all VMs:
- Memory consumption drops drastically
- It becomes feasible to run **400,000+ VMs** simultaneously
- Luau's sandbox ensures isolation between VMs

### Lifecycle

```
VirtualMachine::new()
    ├── Generate UUID
    ├── Create OS
    │
    ├── [spawn_process() ...] ← Lua scripts are loaded
    │
    ├── [tick() called repeatedly by the game loop]
    │   └── OS.tick() executes each process
    │
    └── OS.is_finished() → true → VM is removed
```

The VM is **automatically removed** from the game loop when all of its processes finish.

## Relationships

| Component | Relationship |
|---|---|
| `OS` | Each VM has exactly **one** OS |
| `Process` | The VM's OS can have **N** processes |
| `Lua` | All VMs share **one** Lua state |
| Game Loop | The main loop calls `tick()` on each VM per frame |
