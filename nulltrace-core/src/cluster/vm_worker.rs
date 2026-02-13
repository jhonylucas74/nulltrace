#![allow(dead_code)]
use super::lua_api::context::{SpawnQueueItem, VmContext};
use super::net::packet::Packet;
use super::vm::VirtualMachine;
use mlua::Lua;
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

/// Worker that processes VMs. Uses the same Lua state that created the VMs' processes,
/// so process threads (io.read, io.write) see the correct VmContext (stdin/stdout).
pub struct VmWorker<'a> {
    pub worker_id: usize,
    pub lua: &'a Lua,
}

impl<'a> VmWorker<'a> {
    /// Process a chunk of VMs (ticking their processes)
    ///
    /// SAFETY: This function is called with a mutable slice of VMs that is guaranteed
    /// to be non-overlapping with other workers' slices.
    pub fn process_chunk(
        &self,
        vms: &mut [VirtualMachine],
        active_vms: &[(Uuid, String, Option<super::net::ip::Ipv4Addr>)], // (id, hostname, ip)
    ) -> WorkerResult {
        let mut result = WorkerResult::default();
        result.start_idx = 0;
        result.end_idx = vms.len();

        for vm in vms.iter_mut() {
            // Find VM metadata
            let (hostname, ip) = active_vms
                .iter()
                .find(|(id, _, _)| *id == vm.id)
                .map(|(_, h, ip)| (h.as_str(), *ip))
                .unwrap_or(("unknown", None));

            // Prepare Lua context for this VM
            {
                let mut ctx = self.lua.app_data_mut::<VmContext>().unwrap();
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

            // Tick all processes
            for process in &mut vm.os.processes {
                let prev_uid = process.user_id;
                {
                    let mut ctx = self.lua.app_data_mut::<VmContext>().unwrap();
                    ctx.current_pid = process.id;
                    ctx.current_uid = process.user_id;
                    ctx.current_username = process.username.clone();
                    ctx.set_current_process(
                        process.stdin.clone(),
                        process.stdout.clone(),
                        process.args.clone(),
                        process.forward_stdout_to.clone(),
                    );
                }

                if !process.is_finished() {
                    process.tick();
                    result.process_ticks += 1;
                }

                // Check for uid changes
                if prev_uid != process.user_id {
                    let ctx = self.lua.app_data_ref::<VmContext>().unwrap();
                    if ctx.current_uid != process.user_id {
                        process.user_id = ctx.current_uid;
                        process.username = ctx.current_username.clone();
                    }
                }
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
                let mut ctx = self.lua.app_data_mut::<VmContext>().unwrap();
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
                let mut ctx = self.lua.app_data_mut::<VmContext>().unwrap();

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
                let mut ctx = self.lua.app_data_mut::<VmContext>().unwrap();
                ctx.process_status_map.clear();
                ctx.process_stdout.clear();
            }
        }

        result
    }
}
