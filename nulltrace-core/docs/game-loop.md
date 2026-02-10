# Game Loop & Stress Test

> File: `src/main.rs`

The `stress` binary implements the **main game loop** and serves as a benchmark to validate VM system performance.

## Constants

```rust
const TOTAL_VMS: usize = 400_000;         // Number of VMs created
const PROCESSOS_POR_VM: usize = 2;        // Processes per VM
const FPS: u32 = 60;                      // Target frames per second
const FRAME_TIME: Duration = ~16.6ms;     // Time per frame (1000/60)
```

**Total simultaneous processes**: `400,000 × 2 = 800,000`

## Game Loop Flow

```
 Start
   │
   ▼
 create_lua_state()                     ← 1 single Luau runtime
   │
   ▼
 Create 400,000 VMs                     ← Each with 2 Lua processes
   │
   ▼
 ┌──────────────── LOOP ─────────────┐
 │                                    │
 │  tick_start = now()                │
 │                                    │
 │  For each VM:                      │
 │    ├── vm.os.tick()                │  ← Resume each process
 │    └── If OS finished → remove     │
 │                                    │
 │  elapsed = tick_start.elapsed()    │
 │                                    │
 │  If elapsed < 16.6ms:             │
 │    └── sleep_until(next_frame)     │  ← Wait to maintain 60 FPS
 │  Else:                             │
 │    └── "Late tick!"                │  ← Frame took too long
 │                                    │
 └────────────────────────────────────┘
   │
   ▼
 All VMs finished → Print metrics
```

## Frame Timing

The system uses a **fixed timestep** for stability:

```rust
let mut next_tick = TokioInstant::now();

// Inside the loop:
if elapsed < FRAME_TIME {
    next_tick += FRAME_TIME;
    sleep_until(next_tick).await;    // Wait for the next frame
} else {
    // Tick was late — reset the timer
    next_tick = TokioInstant::now();
}
```

- If the tick finishes **before** 16.6ms → sleep until the next frame
- If the tick **exceeds** 16.6ms → log the delay and recalculate the timer

## Collected Metrics

```rust
struct Metrics {
    vm_count: AtomicUsize,       // Total VMs created
    process_count: AtomicUsize,  // Total processes created
    ticks: AtomicUsize,          // Total ticks executed
}
```

At the end of the benchmark, it prints:
- Total VMs created
- Total processes executed
- Total ticks performed
- Total execution time in seconds

## Example Output

```
Benchmark finalizado!
Total VMs criadas: 400000
Total processos executados: 800000
Total de ticks: 1200000
Tempo de execução: 3.42 segundos
```
