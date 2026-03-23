import { useState, useMemo, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Activity, List } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import {
  getMockCpuPercent,
  getMockMemory,
  getMockDisk,
  MOCK_PROCESSES,
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
/** Shorter interval when Resources tab is open for real CPU/memory graph updates. */
const RESOURCES_POLL_MS = 800;

/** One process from gRPC GetProcessList (VM processes when authenticated). */
interface GrpcProcessEntry {
  pid: number;
  name: string;
  username: string;
  status: string;
  memory_bytes: number;
}

/** Tauri returns camelCase from serde; normalize for the UI. */
function normalizeProcessListResponse(res: Record<string, unknown>): {
  processes: GrpcProcessEntry[];
  disk_used_bytes: number;
  disk_total_bytes: number;
  vm_lua_memory_bytes: number;
  cpu_utilization_percent: number;
  memory_utilization_percent: number;
  error_message: string;
} {
  const n = (a: unknown, b: unknown): number =>
    typeof a === "number" ? a : typeof b === "number" ? b : 0;
  const raw = (res.processes as unknown[]) ?? [];
  const processes: GrpcProcessEntry[] = raw.map((p) => {
    const o = p as Record<string, unknown>;
    return {
      pid: Number(o.pid ?? 0),
      name: String(o.name ?? ""),
      username: String(o.username ?? ""),
      status: String(o.status ?? ""),
      memory_bytes: n(o.memory_bytes, o.memoryBytes),
    };
  });
  return {
    processes,
    disk_used_bytes: n(res.disk_used_bytes, res.diskUsedBytes),
    disk_total_bytes: n(res.disk_total_bytes, res.diskTotalBytes),
    vm_lua_memory_bytes: n(res.vm_lua_memory_bytes, res.vmLuaMemoryBytes),
    cpu_utilization_percent: n(res.cpu_utilization_percent, res.cpuUtilizationPercent),
    memory_utilization_percent: n(res.memory_utilization_percent, res.memoryUtilizationPercent),
    error_message: String(res.error_message ?? res.errorMessage ?? ""),
  };
}

type Section = "resources" | "processes";

const CHART_POINTS = 48;
const CHART_WIDTH = 280;
const CHART_HEIGHT = 44;

/** SVG sparkline: values 0–100, oldest to newest left to right. */
function SparklineChart({ values, className }: { values: number[]; className?: string }) {
  if (values.length < 2) return null;
  // CPU/Memory series are percentages; keep fixed 0-100 scale.
  const min = 0;
  const max = 100;
  const range = max - min;
  const stepX = CHART_WIDTH / (values.length - 1);
  const points = values
    .map((v, i) => {
      const clamped = Math.max(min, Math.min(max, v));
      const x = i * stepX;
      const y = CHART_HEIGHT - ((clamped - min) / range) * (CHART_HEIGHT - 2) - 1;
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

function formatStorageFromGib(gib: number): string {
  if (gib < 1) {
    return `${(gib * 1024).toFixed(1)} MiB`;
  }
  return `${gib.toFixed(1)} GiB`;
}

/** Process list row: memory shown as string (e.g. "1.0 GiB", "0.1 MiB", or "N/A"). */
export interface ProcessRow {
  id: string;
  name: string;
  pid: number;
  cpuPercent: number;
  memoryDisplay: string;
}

/** Derive memory/disk from gRPC or NullCloud or mock. When authenticated, memory used = vm_lua_memory_bytes scaled to nominal; total from sysinfo (nominal). */
function useResourceTotals(
  grpcDisk: { usedBytes: number; totalBytes: number } | null,
  vmLuaMemoryBytes: number | undefined | null,
  totalMemoryMb: number | null
) {
  const nullcloud = useNullCloudOptional();
  const mockMem = getMockMemory();
  const mockDisk = getMockDisk();
  const mockMemRatio = mockMem.totalGib > 0 ? mockMem.usedGib / mockMem.totalGib : 0.25;
  const mockDiskTotal = mockDisk.usedGib + mockDisk.freeGib;
  const mockDiskUsedRatio = mockDiskTotal > 0 ? mockDisk.usedGib / mockDiskTotal : 0.12;

  const memory = (() => {
    // Authenticated: use vm_lua_memory_bytes (real) scaled to nominal for display.
    if (totalMemoryMb != null && vmLuaMemoryBytes != null && vmLuaMemoryBytes >= 0) {
      const totalGib = totalMemoryMb / 1024;
      // real bytes → nominal GiB: ratio 1024:1, so used_nominal_GiB = real_bytes / (1024 * 1024)
      const usedGib = vmLuaMemoryBytes / (1024 * 1024);
      return { usedGib: Math.min(usedGib, totalGib), totalGib };
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
    vm_lua_memory_bytes?: number;
    cpu_utilization_percent?: number;
    memory_utilization_percent?: number;
  } | null>(null);
  const [sysinfoMemoryMb, setSysinfoMemoryMb] = useState<number | null>(null);
  const [grpcError, setGrpcError] = useState<string | null>(null);

  const grpcDisk =
    grpcData != null
      ? { usedBytes: grpcData.disk_used_bytes, totalBytes: grpcData.disk_total_bytes }
      : null;
  const resourceTotals = useResourceTotals(
    grpcDisk,
    grpcData?.vm_lua_memory_bytes,
    sysinfoMemoryMb
  );

  const [cpuHistory, setCpuHistory] = useState<number[]>(() => Array(CHART_POINTS).fill(0));
  const [memoryHistory, setMemoryHistory] = useState<number[]>(() => Array(CHART_POINTS).fill(0));
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
    // Omit CPU/memory utilization when on Processes tab (server returns 0 for those fields).
    const omitResourceMetrics = section === "processes";
    try {
      const res = await invoke<Record<string, unknown>>("grpc_get_process_list", {
        args: {
          token,
          omitResourceMetrics,
        },
      });
      const out = normalizeProcessListResponse(res);
      if (out.error_message === "UNAUTHENTICATED") {
        logout();
        return;
      }
      if (out.error_message) {
        setGrpcError(out.error_message);
        return;
      }
      setGrpcData({
        processes: out.processes,
        disk_used_bytes: out.disk_used_bytes,
        disk_total_bytes: out.disk_total_bytes,
        vm_lua_memory_bytes: out.vm_lua_memory_bytes,
        cpu_utilization_percent: out.cpu_utilization_percent,
        memory_utilization_percent: out.memory_utilization_percent,
      });
    } catch (e) {
      setGrpcError(e instanceof Error ? e.message : String(e));
    }
  }, [playerId, token, logout, section]);

  const fetchSysinfo = useCallback(async () => {
    if (!playerId || !token) return;
    const tauri = (window as unknown as { __TAURI__?: unknown }).__TAURI__;
    if (!tauri) return;
    try {
      const res = await invoke<{ cpu_cores: number; memory_mb: number; disk_mb: number; error_message: string }>(
        "grpc_sysinfo",
        { token }
      );
      if (!res.error_message) {
        setSysinfoMemoryMb(res.memory_mb);
      }
    } catch {
      setSysinfoMemoryMb(null);
    }
  }, [playerId, token]);

  // Base poll for process list and sysinfo (processes tab, disk, memory totals).
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

  // Resources tab: when authenticated, poll more frequently for real CPU/memory graphs.
  // When not authenticated, use mock random walk for demo.
  useEffect(() => {
    if (section !== "resources") return;
    const authenticated = Boolean(playerId && token);
    if (authenticated) {
      // Poll and append real utilization to history.
      const poll = async () => {
        const tauri = (window as unknown as { __TAURI__?: unknown }).__TAURI__;
        if (!tauri) return;
        try {
          const res = await invoke<Record<string, unknown>>("grpc_get_process_list", {
            args: {
              token,
              omitResourceMetrics: false,
            },
          });
          const out = normalizeProcessListResponse(res);
          if (out.error_message) return;
          setCpuHistory((prev) => [...prev.slice(1), out.cpu_utilization_percent]);
          setMemoryHistory((prev) => [...prev.slice(1), out.memory_utilization_percent]);
        } catch {
          // Ignore fetch errors for graph updates.
        }
      };
      poll();
      const t = setInterval(poll, RESOURCES_POLL_MS);
      return () => clearInterval(t);
    }
    // Not authenticated: mock random walk for demo.
    const t = setInterval(() => {
      baseCpu.current = Math.max(0, Math.min(100, baseCpu.current + (Math.random() - 0.5) * 8));
      baseMemPct.current = Math.max(0, Math.min(100, baseMemPct.current + (Math.random() - 0.5) * 3));
      setCpuHistory((prev) => [...prev.slice(1), baseCpu.current]);
      setMemoryHistory((prev) => [...prev.slice(1), baseMemPct.current]);
    }, RESOURCES_POLL_MS);
    return () => clearInterval(t);
  }, [section, playerId, token]);

  const cpuPercent = cpuHistory[cpuHistory.length - 1] ?? (playerId && token ? 0 : getMockCpuPercent());
  const memory = resourceTotals.memory;
  const disk = resourceTotals.disk;
  const diskTotal = resourceTotals.diskTotal;
  const diskUsedPercent = diskTotal > 0 ? (disk.usedGib / diskTotal) * 100 : 0;
  const diskUsedDisplay = formatStorageFromGib(disk.usedGib);
  const diskFreeDisplay = formatStorageFromGib(disk.freeGib);

  const processes: ProcessRow[] = useMemo(() => {
    if (grpcData != null) {
      return grpcData.processes
        .filter((p) => p.status === "running")
        .map((p) => ({
          id: `pid-${p.pid}`,
          name: p.name,
          pid: p.pid,
          cpuPercent: 0,
          memoryDisplay: `${(p.memory_bytes / (1024 * 1024)).toFixed(1)} MiB`,
        }));
    }
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
    [...MOCK_PROCESSES].sort((a, b) => b.cpuPercent - a.cpuPercent).forEach((p) => {
      rows.push({
        id: p.id,
        name: p.name,
        pid: p.pid,
        cpuPercent: p.cpuPercent,
        memoryDisplay: `${p.memoryMb.toFixed(1)} MiB`,
      });
    });
    return rows;
  }, [windows, grpcData]);

  const { t } = useTranslation("systemmonitor");

  return (
    <div className={styles.appWithSidebar}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarSection}>{t("sidebar_monitor")}</div>
        <button
          type="button"
          className={`${styles.navItem} ${section === "resources" ? styles.navItemActive : ""}`}
          onClick={() => setSection("resources")}
        >
          <span className={styles.navIcon}>
            <Activity size={18} />
          </span>
          {t("nav_resources")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "processes" ? styles.navItemActive : ""}`}
          onClick={() => setSection("processes")}
        >
          <span className={styles.navIcon}>
            <List size={18} />
          </span>
          {t("nav_processes")}
        </button>
      </aside>
      <main className={styles.main}>
        {section === "resources" && (
          <>
            <h2 className={styles.mainTitle}>{t("title_resources")}</h2>
            <p className={styles.mainSubtitle}>{t("subtitle_resources")}</p>
            <div className={styles.resourceGrid}>
              <div className={styles.resourceBlock}>
                <div className={styles.resourceRow}>
                  <span className={styles.resourceLabel}>{t("label_cpu")}</span>
                  <span className={styles.resourceValue}>{cpuPercent.toFixed(1)}%</span>
                </div>
                <div className={styles.resourceChart}>
                  <SparklineChart values={cpuHistory} className={styles.sparklineCpu} />
                </div>
              </div>
              <div className={styles.resourceBlock}>
                <div className={styles.resourceRow}>
                  <span className={styles.resourceLabel}>{t("label_memory")}</span>
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
                  <span className={styles.resourceLabel}>{t("label_disk")}</span>
                  <span className={styles.resourceValue}>
                    {diskUsedDisplay} used, {diskFreeDisplay} free
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
            <h2 className={styles.mainTitle}>{t("title_processes")}</h2>
            <p className={styles.mainSubtitle}>
              {grpcData != null
                ? t("subtitle_loaded")
                : token
                  ? grpcError
                    ? t("subtitle_error", { error: grpcError })
                    : t("subtitle_loading")
                  : t("subtitle_login")}
            </p>
            <div className={styles.processHeader}>
              <span className={styles.processHeaderName}>{t("header_name")}</span>
              <span className={styles.processHeaderPid}>{t("header_pid")}</span>
              <span className={styles.processHeaderCpu}>{t("header_cpu")}</span>
              <span className={styles.processHeaderMemory}>{t("header_memory")}</span>
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
