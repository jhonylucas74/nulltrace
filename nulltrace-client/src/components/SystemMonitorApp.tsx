import { useState, useMemo, useEffect, useRef, useCallback } from "react";
import { Activity, List } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import {
  getMockCpuPercent,
  getMockMemory,
  getMockDisk,
  MOCK_PROCESSES,
  OS_BASELINE_GIB,
  OS_KERNEL_GIB,
  OS_DESKTOP_UI_GIB,
} from "../lib/systemMonitorData";
import { getAppByType } from "../lib/appList";
import type { WindowType } from "../contexts/WindowManagerContext";
import { useAuth } from "../contexts/AuthContext";
import { useNullCloudOptional } from "../contexts/NullCloudContext";
import { useWindowManager } from "../contexts/WindowManagerContext";
import styles from "./SystemMonitorApp.module.css";

const PROCESS_LIST_POLL_MS = 3000;

/** One process from gRPC GetProcessList (VM processes when authenticated). */
interface GrpcProcessEntry {
  pid: number;
  name: string;
  username: string;
  status: string;
  memory_bytes: number;
}

type Section = "resources" | "processes";

const CHART_POINTS = 48;
const CHART_WIDTH = 280;
const CHART_HEIGHT = 44;

/** SVG sparkline: values 0–100, oldest to newest left to right. */
function SparklineChart({ values, className }: { values: number[]; className?: string }) {
  if (values.length < 2) return null;
  const max = Math.max(1, ...values);
  const min = Math.min(0, ...values);
  const range = max - min || 1;
  const stepX = CHART_WIDTH / (values.length - 1);
  const points = values
    .map((v, i) => {
      const x = i * stepX;
      const y = CHART_HEIGHT - ((v - min) / range) * (CHART_HEIGHT - 2) - 1;
      return `${x},${y}`;
    })
    .join(" ");
  return (
    <svg
      className={className}
      viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
      width="100%"
      height={CHART_HEIGHT}
      preserveAspectRatio="none"
      aria-hidden
    >
      <polyline
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        points={points}
      />
    </svg>
  );
}

/** Process list row: memory shown as string (e.g. "1.0 GiB", "0.1 MiB", or "N/A"). */
export interface ProcessRow {
  id: string;
  name: string;
  pid: number;
  cpuPercent: number;
  memoryDisplay: string;
}

/** Derive memory/disk from NullCloud localMachine or fallback to mock. When authenticated with gRPC process list and sysinfo, memory used = OS_BASELINE_GIB + sum(process memory); total from sysinfo. */
function useResourceTotals(
  grpcDisk: { usedBytes: number; totalBytes: number } | null,
  grpcProcesses: GrpcProcessEntry[] | null,
  totalMemoryMb: number | null
) {
  const nullcloud = useNullCloudOptional();
  const mockMem = getMockMemory();
  const mockDisk = getMockDisk();
  const mockMemRatio = mockMem.totalGib > 0 ? mockMem.usedGib / mockMem.totalGib : 0.25;
  const mockDiskTotal = mockDisk.usedGib + mockDisk.freeGib;
  const mockDiskUsedRatio = mockDiskTotal > 0 ? mockDisk.usedGib / mockDiskTotal : 0.12;

  const memory = (() => {
    if (totalMemoryMb != null && grpcProcesses != null) {
      const totalGib = totalMemoryMb / 1024;
      const processSumGib = grpcProcesses.reduce((s, p) => s + p.memory_bytes, 0) / (1024 ** 3);
      const usedGib = Math.min(OS_BASELINE_GIB + processSumGib, totalGib);
      return { usedGib, totalGib };
    }
    if (nullcloud) {
      return {
        usedGib: nullcloud.localMachine.ramGib * mockMemRatio,
        totalGib: nullcloud.localMachine.ramGib,
      };
    }
    return mockMem;
  })();

  const disk = grpcDisk
    ? {
        usedGib: grpcDisk.usedBytes / (1024 * 1024 * 1024),
        freeGib: (grpcDisk.totalBytes - grpcDisk.usedBytes) / (1024 * 1024 * 1024),
      }
    : nullcloud
      ? (() => {
          const diskUsed = nullcloud.localMachine.diskTotalGib * mockDiskUsedRatio;
          const diskFree = nullcloud.localMachine.diskTotalGib - diskUsed;
          return { usedGib: diskUsed, freeGib: diskFree };
        })()
      : mockDisk;

  const diskTotal = grpcDisk
    ? grpcDisk.totalBytes / (1024 * 1024 * 1024)
    : nullcloud
      ? nullcloud.localMachine.diskTotalGib
      : mockDiskTotal;

  return { memory, disk, diskTotal };
}

/** Synthetic PID base for UI-only window processes (avoid clash with VM PIDs). */
const WINDOW_PID_BASE = 1000;

export default function SystemMonitorApp() {
  const { playerId, token, logout } = useAuth();
  const { windows } = useWindowManager();
  const [section, setSection] = useState<Section>("resources");
  const [grpcData, setGrpcData] = useState<{
    processes: GrpcProcessEntry[];
    disk_used_bytes: number;
    disk_total_bytes: number;
  } | null>(null);
  const [sysinfoMemoryMb, setSysinfoMemoryMb] = useState<number | null>(null);
  const [grpcError, setGrpcError] = useState<string | null>(null);

  const grpcDisk =
    grpcData != null
      ? { usedBytes: grpcData.disk_used_bytes, totalBytes: grpcData.disk_total_bytes }
      : null;
  const resourceTotals = useResourceTotals(
    grpcDisk,
    grpcData?.processes ?? null,
    sysinfoMemoryMb
  );

  const [cpuHistory, setCpuHistory] = useState<number[]>(() => Array(CHART_POINTS).fill(getMockCpuPercent()));
  const [memoryHistory, setMemoryHistory] = useState<number[]>(() => {
    const pct = resourceTotals.memory.totalGib > 0
      ? (resourceTotals.memory.usedGib / resourceTotals.memory.totalGib) * 100
      : 0;
    return Array(CHART_POINTS).fill(pct);
  });
  const baseCpu = useRef(getMockCpuPercent());
  const memPctInitial = useMemo(() => {
    const t = resourceTotals.memory;
    return t.totalGib > 0 ? (t.usedGib / t.totalGib) * 100 : 0;
  }, [resourceTotals.memory.totalGib, resourceTotals.memory.usedGib]);
  const baseMemPct = useRef(memPctInitial);

  const fetchProcessList = useCallback(async () => {
    if (!playerId || !token) return;
    const tauri = (window as unknown as { __TAURI__?: unknown }).__TAURI__;
    if (!tauri) return;
    setGrpcError(null);
    try {
      const res = await invoke<{
        processes: GrpcProcessEntry[];
        disk_used_bytes: number;
        disk_total_bytes: number;
        error_message: string;
      }>("grpc_get_process_list", { playerId, token });
      if (res.error_message === "UNAUTHENTICATED") {
        logout();
        return;
      }
      if (res.error_message) {
        setGrpcError(res.error_message);
        return;
      }
      setGrpcData({
        processes: Array.isArray(res.processes) ? res.processes : [],
        disk_used_bytes: res.disk_used_bytes ?? 0,
        disk_total_bytes: res.disk_total_bytes ?? 0,
      });
    } catch (e) {
      setGrpcError(e instanceof Error ? e.message : String(e));
    }
  }, [playerId, token, logout]);

  const fetchSysinfo = useCallback(async () => {
    if (!playerId || !token) return;
    const tauri = (window as unknown as { __TAURI__?: unknown }).__TAURI__;
    if (!tauri) return;
    try {
      const res = await invoke<{ cpu_cores: number; memory_mb: number; disk_mb: number; error_message: string }>(
        "grpc_sysinfo",
        { playerId, token }
      );
      if (!res.error_message) {
        setSysinfoMemoryMb(res.memory_mb);
      }
    } catch {
      setSysinfoMemoryMb(null);
    }
  }, [playerId, token]);

  useEffect(() => {
    if (!playerId || !token) {
      setSysinfoMemoryMb(null);
      return;
    }
    fetchProcessList();
    fetchSysinfo();
    const t = setInterval(() => {
      fetchProcessList();
      fetchSysinfo();
    }, PROCESS_LIST_POLL_MS);
    return () => clearInterval(t);
  }, [playerId, token, fetchProcessList, fetchSysinfo]);

  useEffect(() => {
    if (section !== "resources") return;
    const t = setInterval(() => {
      baseCpu.current = Math.max(0, Math.min(100, baseCpu.current + (Math.random() - 0.5) * 8));
      baseMemPct.current = Math.max(0, Math.min(100, baseMemPct.current + (Math.random() - 0.5) * 3));
      setCpuHistory((prev) => [...prev.slice(1), baseCpu.current]);
      setMemoryHistory((prev) => [...prev.slice(1), baseMemPct.current]);
    }, 800);
    return () => clearInterval(t);
  }, [section]);

  const cpuPercent = cpuHistory[cpuHistory.length - 1] ?? getMockCpuPercent();
  const memory = resourceTotals.memory;
  const disk = resourceTotals.disk;
  const diskTotal = resourceTotals.diskTotal;
  const diskUsedPercent = diskTotal > 0 ? (disk.usedGib / diskTotal) * 100 : 0;

  const processes: ProcessRow[] = useMemo(() => {
    const rows: ProcessRow[] = [];
    rows.push({
      id: "os-kernel",
      name: "kernel",
      pid: 0,
      cpuPercent: 0,
      memoryDisplay: `${OS_KERNEL_GIB.toFixed(1)} GiB`,
    });
    rows.push({
      id: "os-desktop-ui",
      name: "desktop-ui",
      pid: 1,
      cpuPercent: 0,
      memoryDisplay: `${OS_DESKTOP_UI_GIB.toFixed(1)} GiB`,
    });
    windows.forEach((win, i) => {
      const label = getAppByType(win.type as WindowType)?.label ?? win.type;
      rows.push({
        id: `win-${win.id}`,
        name: label,
        pid: WINDOW_PID_BASE + i,
        cpuPercent: 0,
        memoryDisplay: "N/A",
      });
    });
    if (grpcData != null) {
      grpcData.processes.forEach((p) => {
        rows.push({
          id: `pid-${p.pid}`,
          name: p.name,
          pid: p.pid,
          cpuPercent: 0,
          memoryDisplay: `${(p.memory_bytes / (1024 * 1024)).toFixed(1)} MiB`,
        });
      });
    } else {
      [...MOCK_PROCESSES].sort((a, b) => b.cpuPercent - a.cpuPercent).forEach((p) => {
        rows.push({
          id: p.id,
          name: p.name,
          pid: p.pid,
          cpuPercent: p.cpuPercent,
          memoryDisplay: `${p.memoryMb.toFixed(1)} MiB`,
        });
      });
    }
    return rows;
  }, [windows, grpcData]);

  return (
    <div className={styles.appWithSidebar}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarSection}>Monitor</div>
        <button
          type="button"
          className={`${styles.navItem} ${section === "resources" ? styles.navItemActive : ""}`}
          onClick={() => setSection("resources")}
        >
          <span className={styles.navIcon}>
            <Activity size={18} />
          </span>
          Resources
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "processes" ? styles.navItemActive : ""}`}
          onClick={() => setSection("processes")}
        >
          <span className={styles.navIcon}>
            <List size={18} />
          </span>
          Processes
        </button>
      </aside>
      <main className={styles.main}>
        {section === "resources" && (
          <>
            <h2 className={styles.mainTitle}>Resources</h2>
            <p className={styles.mainSubtitle}>CPU, memory, and disk usage. No GPU.</p>
            <div className={styles.resourceGrid}>
              <div className={styles.resourceBlock}>
                <div className={styles.resourceRow}>
                  <span className={styles.resourceLabel}>CPU</span>
                  <span className={styles.resourceValue}>{cpuPercent.toFixed(1)}%</span>
                </div>
                <div className={styles.resourceChart}>
                  <SparklineChart values={cpuHistory} className={styles.sparklineCpu} />
                </div>
              </div>
              <div className={styles.resourceBlock}>
                <div className={styles.resourceRow}>
                  <span className={styles.resourceLabel}>Memory</span>
                  <span className={styles.resourceValue}>
                    {memory.usedGib.toFixed(1)} / {memory.totalGib.toFixed(1)} GiB
                  </span>
                </div>
                <div className={styles.resourceChart}>
                  <SparklineChart values={memoryHistory} className={styles.sparklineMemory} />
                </div>
              </div>
              <div className={styles.resourceBlock}>
                <div className={styles.resourceRow}>
                  <span className={styles.resourceLabel}>Disk</span>
                  <span className={styles.resourceValue}>
                    {disk.usedGib.toFixed(1)} GiB used, {disk.freeGib.toFixed(1)} GiB free
                  </span>
                </div>
                <div className={styles.progressTrack}>
                  <div
                    className={styles.progressFill}
                    style={{ width: `${diskUsedPercent}%` }}
                    role="progressbar"
                    aria-valuenow={Math.round(diskUsedPercent)}
                    aria-valuemin={0}
                    aria-valuemax={100}
                  />
                </div>
              </div>
            </div>
          </>
        )}
        {section === "processes" && (
          <>
            <h2 className={styles.mainTitle}>Processes</h2>
            <p className={styles.mainSubtitle}>
              {grpcData != null
                ? "VM processes (refreshed every few seconds)."
                : token
                  ? grpcError
                    ? `Error: ${grpcError}`
                    : "Loading…"
                  : "Log in to see your VM processes."}
            </p>
            <div className={styles.processHeader}>
              <span className={styles.processHeaderName}>Name</span>
              <span className={styles.processHeaderPid}>PID</span>
              <span className={styles.processHeaderCpu}>CPU %</span>
              <span className={styles.processHeaderMemory}>Memory</span>
            </div>
            <ul className={styles.processList}>
              {processes.map((proc) => (
                <li key={proc.id} className={styles.processRow}>
                  <span className={styles.processName}>{proc.name}</span>
                  <span className={styles.processPid}>{proc.pid}</span>
                  <span className={styles.processCpu}>{proc.cpuPercent.toFixed(1)}</span>
                  <span className={styles.processMemory}>{proc.memoryDisplay}</span>
                </li>
              ))}
            </ul>
          </>
        )}
      </main>
    </div>
  );
}
