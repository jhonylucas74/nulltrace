#![allow(dead_code)]
use super::lua_api::context::{SpawnQueueItem, VmContext};
use super::net::packet::Packet;
use super::vm::VirtualMachine;
use uuid::Uuid;

/// Task sent to a worker thread
pub enum WorkerTask {
    /// Process a chunk of VMs (start..end indices)
    ProcessChunk { start_idx: usize, end_idx: usize },
    /// Shutdown the worker
    Shutdown,
}

/// Result returned by a worker after processing VMs
#[derive(Default)]
pub struct WorkerResult {
    pub start_idx: usize,
    pub end_idx: usize,
    pub process_ticks: u64,
    /// VM ids that hit memory limit; caller should reset their Lua state.
    pub memory_exceeded_vms: Vec<Uuid>,
    /// Network packets accumulated from all VMs in this chunk
    pub net_outbound: Vec<(Uuid, Packet)>,  // (vm_id, packet)
    /// Listening ports to register: (vm_id, port, pid)
    pub pending_listen: Vec<(Uuid, u16, u64)>,
    /// Ephemeral ports to register: (vm_id, port)
    pub pending_ephemeral_register: Vec<(Uuid, u16)>,
    /// Ephemeral ports to unregister: (vm_id, port)
    pub pending_ephemeral_unregister: Vec<(Uuid, u16)>,
    /// PIDs of finished processes by VM: (vm_id, pids)
    pub finished_pids: Vec<(Uuid, Vec<u64>)>,
    /// Stdout of finished processes: (vm_id, pid, stdout)
    pub finished_stdout: Vec<(Uuid, u64, String)>,
    /// Spawn queue items: (vm_id, spawn_item)
    pub spawn_queue: Vec<(Uuid, SpawnQueueItem)>,
    /// Stdin inject queue: (vm_id, pid, line)
    pub stdin_inject_queue: Vec<(Uuid, u64, String)>,
}

// VmWorkerHandle removed - not needed for current implementation

/// Worker that processes VMs. Each VM has its own Lua state; VmContext is set on that VM's Lua.
pub struct VmWorker {
    pub worker_id: usize,
}

impl VmWorker {
    /// Process a chunk of VMs (one process per VM per tick, round-robin).
    /// Only processes VMs at executable_indices.
    ///
    /// SAFETY: This function is called with a mutable slice of VMs that is guaranteed
    /// to be non-overlapping with other workers' slices.
    pub fn process_chunk(
        &self,
        vms: &mut [VirtualMachine],
        executable_indices: &[usize],
        active_vms: &[(Uuid, String, Option<super::net::ip::Ipv4Addr>)], // (id, hostname, ip)
    ) -> WorkerResult {
        let mut result = WorkerResult::default();
        result.start_idx = 0;
        result.end_idx = vms.len();

        for &vm_idx in executable_indices {
            let vm = &mut vms[vm_idx];
            let lua = &vm.lua;

            // Find VM metadata
            let (hostname, ip) = active_vms
                .iter()
                .find(|(id, _, _)| *id == vm.id)
                .map(|(_, h, ip)| (h.as_str(), *ip))
                .unwrap_or(("unknown", None));

            // Prepare Lua context for this VM
            {
                let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                ctx.set_vm(vm.id, hostname, ip);
                if let Some(nic) = &vm.nic {
                    ctx.set_port_owners(nic.get_port_owners());
                }
                ctx.next_pid = vm.os.next_process_id();

                // Snapshot process status and stdout
                for p in &vm.os.processes {
                    let status = if p.is_finished() { "finished" } else { "running" };
                    ctx.process_status_map.insert(p.id, status.to_string());
                    if let Ok(guard) = p.stdout.lock() {
                        ctx.process_stdout.insert(p.id, guard.clone());
                    }
                }
                ctx.merge_last_stdout_of_finished();

                // Load inbound packets into context
                if let Some(nic) = &mut vm.nic {
                    while let Some(pkt) = nic.recv() {
                        ctx.net_inbound.push_back(pkt);
                    }
                }

                // Swap in this VM's connection state
                std::mem::swap(&mut ctx.connections, &mut vm.connections);
                std::mem::swap(&mut ctx.next_connection_id, &mut vm.next_connection_id);
                if let Some(nic) = &mut vm.nic {
                    ctx.sync_connection_inbounds_from_nic(nic);
                }
            }

            // Tick one process (round-robin)
            let process_idx = vm.os.get_next_tick_index();
            if let Some(idx) = process_idx {
                let (pid, stdin, stdout, args, forward_stdout_to, username) = {
                    let p = &vm.os.processes[idx];
                    (
                        p.id,
                        p.stdin.clone(),
                        p.stdout.clone(),
                        p.args.clone(),
                        p.forward_stdout_to.clone(),
                        p.username.clone(),
                    )
                };
                {
                    let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                    ctx.current_pid = pid;
                    ctx.current_uid = vm.os.processes[idx].user_id;
                    ctx.current_username = username.clone();
                    ctx.set_current_process(stdin, stdout, args, forward_stdout_to);
                }
                if let Err(e) = vm.os.tick_process_at(idx) {
                    if matches!(e, mlua::Error::MemoryError(_)) {
                        println!("[cluster] VM {} exceeded memory limit (1 MB)", vm.id);
                        result.memory_exceeded_vms.push(vm.id);
                    }
                }

                // Check for uid changes (process may have called os.setuid in Lua)
                {
                    let ctx = lua.app_data_ref::<VmContext>().unwrap();
                    if let Some(p) = vm.os.processes.iter_mut().find(|pr| pr.id == pid) {
                        if ctx.current_uid != p.user_id {
                            p.user_id = ctx.current_uid;
                            p.username = ctx.current_username.clone();
                        }
                    }
                }
                result.process_ticks += 1;
            }

            // Capture finished process stdout
            let finished_pids: Vec<u64> = vm
                .os
                .processes
                .iter()
                .filter(|p| p.is_finished())
                .map(|p| p.id)
                .collect();

            {
                let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                for p in &vm.os.processes {
                    if p.is_finished() {
                        if let Ok(guard) = p.stdout.lock() {
                            ctx.last_stdout_of_finished.insert(p.id, guard.clone());
                            result.finished_stdout.push((vm.id, p.id, guard.clone()));
                        }
                    }
                }
                for pid in &finished_pids {
                    ctx.close_connections_for_pid(*pid);
                }
            }

            // Remove finished processes
            vm.os.processes.retain(|p| !p.is_finished());
            if let Some(nic) = &mut vm.nic {
                for pid in &finished_pids {
                    nic.unlisten_pid(*pid);
                }
            }

            if !finished_pids.is_empty() {
                result.finished_pids.push((vm.id, finished_pids));
            }

            // Collect network operations, spawn queue, and stdin inject from context
            {
                let mut ctx = lua.app_data_mut::<VmContext>().unwrap();

                // Collect outbound packets
                for pkt in ctx.net_outbound.drain(..) {
                    result.net_outbound.push((vm.id, pkt));
                }

                // Collect listen requests
                for (port, pid) in ctx.pending_listen.drain(..) {
                    result.pending_listen.push((vm.id, port, pid));
                }

                // Collect ephemeral port registrations
                for port in ctx.pending_ephemeral_register.drain(..) {
                    result.pending_ephemeral_register.push((vm.id, port));
                }

                for port in ctx.pending_ephemeral_unregister.drain(..) {
                    result.pending_ephemeral_unregister.push((vm.id, port));
                }

                // Collect spawn queue (to be processed in main thread with async fs access)
                for item in ctx.spawn_queue.drain(..) {
                    result.spawn_queue.push((vm.id, item));
                }

                // Collect stdin inject queue
                for (pid, line) in ctx.stdin_inject_queue.drain(..) {
                    result.stdin_inject_queue.push((vm.id, pid, line));
                }

                // Return unconsumed inbound packets to NIC
                if let Some(nic) = &mut vm.nic {
                    for pkt in ctx.net_inbound.drain(..) {
                        nic.deliver(pkt);
                    }
                }

                // Swap back this VM's connection state
                std::mem::swap(&mut ctx.connections, &mut vm.connections);
                std::mem::swap(&mut ctx.next_connection_id, &mut vm.next_connection_id);
            }

            // Clear context for next VM
            {
                let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                ctx.process_status_map.clear();
                ctx.process_stdout.clear();
            }
        }

        result
    }
}
