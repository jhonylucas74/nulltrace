# OS & Scheduler

> File: `src/cluster/os.rs`

The `OS` is the **simulated operating system** for each VM. It manages process creation, execution, and removal.

## Structure

```rust
pub struct OS<'a> {
    pub processes: Vec<Process>,     // List of active processes
    next_process_id: AtomicU64,      // Incremental PID generator
    is_finished: bool,               // Whether all processes have finished
    lua: &'a Lua,                    // Reference to the shared Lua runtime
}
```

## Luau Sandbox

The `create_lua_state()` function configures the Luau runtime with security:

```rust
pub fn create_lua_state() -> Lua {
    let lua = Lua::new();
    lua.sandbox(true);        // Enable sandbox — blocks I/O, filesystem access, etc.

    // Replace print() with a Rust function (currently silenced)
    lua.globals().set("print", rust_print);

    // Configure interrupts: every 2 instructions, force a Yield
    lua.set_interrupt(move |_| {
        if count.fetch_add(1, Ordering::Relaxed) % 2 == 0 {
            return Ok(VmState::Yield);
        }
        Ok(VmState::Continue)
    });

    lua
}
```

### Interrupt Mechanism (Preemptive Scheduling)

`set_interrupt` is the **heart of the scheduler**. Here's how it works:

1. Luau calls the interrupt handler every N VM instructions
2. The handler counts calls with an `AtomicU64`
3. Every **2 calls**, it returns `VmState::Yield` — pausing the Lua thread
4. On odd calls, it returns `VmState::Continue` — letting execution proceed

This creates **cooperative time-slicing**: each process executes a bit and then yields control, preventing any single script from monopolizing the CPU.

```
Process A: [execute] [yield] [execute] [yield] ...
Process B: [execute] [yield] [execute] [yield] ...
                 ↑                ↑
          Interrupt handler forces the pause
```

## Key Methods

### `spawn_process(lua_code: &str)`

Creates a new process from Lua code:

```rust
vm.os.spawn_process(r#"
    local x = 1 + 2
    print(x)
"#);
```

1. Generates an **incremental PID** via `AtomicU64`
2. Creates a `Process` (compiles the Lua code into a thread)
3. Adds it to the process list

### `tick()`

Called by the game loop **once per frame**:

```rust
pub fn tick(&mut self) {
    // 1. Resume each process that hasn't finished
    for process in &mut self.processes {
        if !process.is_finished() {
            process.tick();
        }
    }

    // 2. Remove finished processes
    self.processes.retain(|p| !p.is_finished());

    // 3. If all are done, mark the OS as finished
    self.is_finished = self.processes.iter().all(|proc| proc.is_finished());
}
```

### `run()` (utility)

Runs the OS in a loop until all processes finish or it hits 1000 ticks (safety limit):

```rust
pub fn run(&mut self) -> u128 {
    // Returns total time in milliseconds
}
```

## Complete Flow

```
create_lua_state()
    │
    ▼
OS::new(&lua)
    │
    ├── spawn_process("script_1.lua")  → PID 1
    ├── spawn_process("script_2.lua")  → PID 2
    │
    ▼
tick()  ← Frame 1
    ├── Process 1: resume → execute → yield
    ├── Process 2: resume → execute → yield
    └── Remove finished processes
    │
    ▼
tick()  ← Frame 2
    ├── Process 1: resume → execute → finish ✓
    ├── Process 2: resume → execute → yield
    └── Remove Process 1
    │
    ▼
tick()  ← Frame 3
    ├── Process 2: resume → execute → finish ✓
    └── is_finished = true → VM will be removed
```
