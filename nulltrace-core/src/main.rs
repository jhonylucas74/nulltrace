#[path = "cluster/process.rs"]
mod process;
#[path = "cluster/os.rs"]
mod os;
#[path = "cluster/net/mod.rs"]
mod net;
#[path = "cluster/vm.rs"]
mod vm;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

const TOTAL_VMS: usize = 1_000;
const PROCESSOS_POR_VM: usize = 1;
const TEST_DURATION_SECS: u64 = 300; // 5 minutes
const FPS: u32 = 60;
const FRAME_TIME: Duration = Duration::from_millis(1000 / FPS as u64);

struct Metrics {
    vm_count: AtomicUsize,
    process_count: AtomicUsize,
    total_ticks: AtomicUsize,
    process_ticks: AtomicUsize,
    slow_ticks: AtomicUsize,
}

#[tokio::main]
async fn main() {
    let start = Instant::now();

    let metrics = Arc::new(Metrics {
        vm_count: AtomicUsize::new(0),
        process_count: AtomicUsize::new(0),
        total_ticks: AtomicUsize::new(0),
        process_ticks: AtomicUsize::new(0),
        slow_ticks: AtomicUsize::new(0),
    });

    let mut vms = Vec::with_capacity(TOTAL_VMS);

    for _ in 0..TOTAL_VMS {
        let lua = os::create_vm_lua_state_minimal().expect("Failed to create VM Lua state");
        let mut vm = vm::VirtualMachine::new(lua);
        metrics.vm_count.fetch_add(1, Ordering::Relaxed);

        for _ in 0..PROCESSOS_POR_VM {
            vm.os.spawn_process(
                &vm.lua,
                r#"
local count = 0
while true do
    count = count + 1
    print("VM tick: " .. count)
end
            "#,
                vec![],
                0,
                "root",
            );
            metrics.process_count.fetch_add(1, Ordering::Relaxed);
        }

        vms.push(vm);
    }

    // Loop principal de jogo
    let mut tick_durations: Vec<Duration> = Vec::new();
    let mut min_duration = Duration::MAX;
    let mut max_duration = Duration::ZERO;

    while start.elapsed() < Duration::from_secs(TEST_DURATION_SECS) {
        let tick_start = Instant::now();

        // Tick em cada VM, remove as que terminaram
        vms.retain_mut(|vm| {
            if let Err(e) = vm.os.tick() {
                if matches!(e, mlua::Error::MemoryError(_)) {
                    println!("[stress] VM {} exceeded memory limit (1 MB), resetting state", vm.id);
                    let _ = vm.reset_lua_state(|| os::create_vm_lua_state_minimal());
                }
            }
            // Count process ticks (1 tick per process per game tick)
            metrics.process_ticks.fetch_add(vm.os.processes.len(), Ordering::Relaxed);
            !vm.os.is_finished()
        });

        // Increment game loop tick counter
        metrics.total_ticks.fetch_add(1, Ordering::Relaxed);

        let tick_duration = tick_start.elapsed();

        // Track statistics
        tick_durations.push(tick_duration);
        min_duration = min_duration.min(tick_duration);
        max_duration = max_duration.max(tick_duration);

        // Count slow ticks (> 16ms threshold)
        if tick_duration > FRAME_TIME {
            metrics.slow_ticks.fetch_add(1, Ordering::Relaxed);
        }
    }

    // Fim
    let duration = start.elapsed();
    let total_ticks = metrics.total_ticks.load(Ordering::Relaxed);
    let total_process_ticks = metrics.process_ticks.load(Ordering::Relaxed);
    let slow_ticks = metrics.slow_ticks.load(Ordering::Relaxed);

    // Calculate percentiles
    tick_durations.sort();
    let p50 = tick_durations[tick_durations.len() / 2];
    let p95 = tick_durations[tick_durations.len() * 95 / 100];
    let p99 = tick_durations[tick_durations.len() * 99 / 100];
    let mean: Duration = tick_durations.iter().sum::<Duration>() / tick_durations.len() as u32;

    println!("\n=== STRESS TEST RESULTS ===\n");
    println!("Configuration:");
    println!("  VMs: {}", vms.len());
    println!("  Processes per VM: {}", PROCESSOS_POR_VM);
    println!("  Total Processes: {}\n", metrics.process_count.load(Ordering::Relaxed));

    // Lua state memory (per-VM heap)
    let total_lua_bytes: usize = vms.iter().map(|vm| vm.lua.used_memory()).sum();
    let avg_lua_bytes = if vms.is_empty() {
        0
    } else {
        total_lua_bytes / vms.len()
    };
    println!("Lua Memory (Per-VM State):");
    println!("  Total: {} bytes ({:.2} MB)", total_lua_bytes, total_lua_bytes as f64 / 1_048_576.0);
    println!("  Average per VM: {} bytes ({:.2} KB)\n", avg_lua_bytes, avg_lua_bytes as f64 / 1024.0);

    println!("Performance:");
    println!("  Duration: {:.2}s", duration.as_secs_f64());
    println!("  Total Game Loop Ticks: {}", total_ticks);
    println!("  Game Loop Ticks/Second: {:.1}", total_ticks as f64 / duration.as_secs_f64());
    println!("  Total Process Ticks: {}", total_process_ticks);
    println!("  Process Ticks/Second: {:.0}\n", total_process_ticks as f64 / duration.as_secs_f64());

    println!("Tick Duration Statistics:");
    println!("  Min: {:?}", min_duration);
    println!("  Max: {:?}", max_duration);
    println!("  Mean: {:?}", mean);
    println!("  Median (p50): {:?}", p50);
    println!("  p95: {:?}", p95);
    println!("  p99: {:?}", p99);
    println!("  Slow ticks (>16ms): {} ({:.1}%)", slow_ticks, (slow_ticks as f64 / total_ticks as f64) * 100.0);
}
