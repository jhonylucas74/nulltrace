import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Server, Activity, Clock, Zap } from "lucide-react";
import styles from "./DashboardOverview.module.css";

interface ClusterStats {
  vm_count: number;
  tick_count: number;
  uptime_secs: number;
  effective_tps: number;
}

export default function DashboardOverview({ token }: { token: string }) {
  const [stats, setStats] = useState<ClusterStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function fetchStats() {
      try {
        const res = await invoke<ClusterStats>("get_cluster_stats", { token });
        if (!cancelled) setStats(res);
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : "Failed to fetch");
      }
    }
    fetchStats();
    const id = setInterval(fetchStats, 2000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [token]);

  if (error) {
    return <p className={styles.error}>{error}</p>;
  }

  if (!stats) {
    return (
      <div className={styles.loading}>
        <p>Loading cluster stats…</p>
      </div>
    );
  }

  const formatUptime = (secs: number) => {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = Math.floor(secs % 60);
    if (h > 0) return `${h}h ${m}m ${s}s`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
  };

  return (
    <div className={styles.root}>
      <h1 className={styles.title}>Dashboard</h1>
      <div className={styles.cards}>
        <div className={styles.card}>
          <Server size={24} className={styles.cardIcon} />
          <div>
            <p className={styles.cardValue}>{stats.vm_count}</p>
            <p className={styles.cardLabel}>Running VMs</p>
          </div>
        </div>
        <div className={styles.card}>
          <Activity size={24} className={styles.cardIcon} />
          <div>
            <p className={styles.cardValue}>{stats.effective_tps.toFixed(1)}</p>
            <p className={styles.cardLabel}>Effective TPS</p>
          </div>
        </div>
        <div className={styles.card}>
          <Clock size={24} className={styles.cardIcon} />
          <div>
            <p className={styles.cardValue}>{formatUptime(stats.uptime_secs)}</p>
            <p className={styles.cardLabel}>Uptime</p>
          </div>
        </div>
        <div className={styles.card}>
          <Zap size={24} className={styles.cardIcon} />
          <div>
            <p className={styles.cardValue}>{stats.tick_count.toLocaleString()}</p>
            <p className={styles.cardLabel}>Total Ticks</p>
          </div>
        </div>
      </div>
    </div>
  );
}
