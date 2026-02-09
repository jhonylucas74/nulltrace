
// mod game;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::time::{sleep_until, Instant as TokioInstant};

const TOTAL_VMS: usize = 400_000;
const PROCESSOS_POR_VM: usize = 2;
const FPS: u32 = 60;
const FRAME_TIME: Duration = Duration::from_millis(1000 / FPS as u64);

struct Metrics {
    vm_count: AtomicUsize,
    process_count: AtomicUsize,
    ticks: AtomicUsize,
}

#[tokio::main]
async fn main() {
    let start = Instant::now();

    let metrics = Arc::new(Metrics {
        vm_count: AtomicUsize::new(0),
        process_count: AtomicUsize::new(0),
        ticks: AtomicUsize::new(0),
    });

    let mut vms = Vec::with_capacity(TOTAL_VMS);

    let lua = game::os::create_lua_state();
    for _ in 0..TOTAL_VMS {
        let mut vm = game::vm::VirtualMachine::new(&lua);
        metrics.vm_count.fetch_add(1, Ordering::Relaxed);

        for _ in 0..PROCESSOS_POR_VM {
            vm.os.spawn_process(
               r#"
-- Define inimigos com vida
local inimigos = {
    { nome = "Goblin", vida = 30 },
    { nome = "Orc", vida = 50 },
    { nome = "Troll", vida = 80 }
}

-- Função para causar dano
local function atacar(inimigo, dano)
    inimigo.vida = inimigo.vida - dano
    print(inimigo.nome .. " sofreu " .. dano .. " de dano. Vida restante: " .. inimigo.vida)
end

-- Loop para atacar cada inimigo
for i = 1, #inimigos do
    local inimigo = inimigos[i]
    
    if inimigo.vida > 0 then
        local dano = 10
        atacar(inimigo, dano)

        if inimigo.vida <= 0 then
            print(inimigo.nome .. " foi derrotado!")
        end
    end
end
            "#,
            );
            metrics.process_count.fetch_add(1, Ordering::Relaxed);
        }

        vms.push(vm);
    }

    // Loop principal de jogo
    let mut next_tick = TokioInstant::now();

    while !vms.is_empty() {
        let tick_start = Instant::now();

        // Tick em cada VM, remove as que terminaram
        vms.retain_mut(|vm| {
            vm.os.tick();
            metrics.ticks.fetch_add(1, Ordering::Relaxed);
            !vm.os.is_finished()
        });

        let elapsed = tick_start.elapsed();
        if elapsed < FRAME_TIME {
            next_tick += FRAME_TIME;
            sleep_until(next_tick).await;
        } else {
            // Tick atrasado, ajusta tempo para o próximo frame
            println!("Tick atrasado meu: {:?}", elapsed);
            next_tick = TokioInstant::now();
        }
    }

    // Fim
    println!("Benchmark finalizado!");
    println!(
        "Total VMs criadas: {}",
        metrics.vm_count.load(Ordering::Relaxed)
    );
    println!(
        "Total processos executados: {}",
        metrics.process_count.load(Ordering::Relaxed)
    );
    println!(
        "Total de ticks: {}",
        metrics.ticks.load(Ordering::Relaxed)
    );

    let duration = start.elapsed();
    println!("Tempo de execução: {:.2} segundos", duration.as_secs_f64());
}
