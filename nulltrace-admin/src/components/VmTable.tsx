import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2 } from "lucide-react";
import styles from "./VmTable.module.css";

interface VmInfo {
  id: string;
  hostname: string;
  dns_name: string;
  ip: string;
  subnet: string;
  gateway: string;
  cpu_cores: number;
  memory_mb: number;
  disk_mb: number;
  owner_id: string;
  real_memory_bytes: number;
  disk_used_bytes: number;
  ticks_per_second: number;
  remaining_ticks: number;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export default function VmTable({ token }: { token: string }) {
  const [vms, setVms] = useState<VmInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    async function fetchVms() {
      try {
        const res = await invoke<{ vms: VmInfo[] }>("list_vms", { token });
        if (!cancelled) setVms(res.vms);
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : "Failed to fetch");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    fetchVms();
    const id = setInterval(fetchVms, 3000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [token]);

  if (loading) {
    return (
      <div className={styles.loading}>
        <Loader2 size={28} className={styles.spinner} aria-hidden />
        <p>Loading VMs…</p>
      </div>
    );
  }

  if (error) {
    return <p className={styles.error}>{error}</p>;
  }

  return (
    <div className={styles.root}>
      <h1 className={styles.title}>Virtual Machines</h1>
      <p className={styles.subtitle}>{vms.length} VM(s) running</p>
      <div className={styles.tableWrap}>
        <table className={styles.table}>
          <thead>
            <tr>
              <th>Hostname</th>
              <th>IP</th>
              <th>Subnet</th>
              <th>CPU</th>
              <th>RAM (nom)</th>
              <th>RAM (real)</th>
              <th>Disk used</th>
              <th>TPS</th>
              <th>Budget</th>
            </tr>
          </thead>
          <tbody>
            {vms.map((vm) => (
              <tr key={vm.id}>
                <td>
                  <span className={styles.hostname}>{vm.dns_name || vm.hostname}</span>
                </td>
                <td className={styles.mono}>{vm.ip || "—"}</td>
                <td className={styles.mono}>{vm.subnet || "—"}</td>
                <td>{vm.cpu_cores}</td>
                <td>{vm.memory_mb} MB</td>
                <td>{formatBytes(vm.real_memory_bytes)}</td>
                <td>{formatBytes(vm.disk_used_bytes)}</td>
                <td>{vm.ticks_per_second}</td>
                <td>{vm.remaining_ticks}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
