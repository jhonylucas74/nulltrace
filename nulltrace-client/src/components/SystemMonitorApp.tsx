import { useState, useMemo, useEffect, useRef } from "react";
import { Activity, List } from "lucide-react";
import {
  getMockCpuPercent,
  getMockMemory,
  getMockDisk,
  MOCK_PROCESSES,
  type MockProcess,
} from "../lib/systemMonitorData";
import { useNullCloudOptional } from "../contexts/NullCloudContext";
import styles from "./SystemMonitorApp.module.css";

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

/** Derive memory/disk from NullCloud localMachine or fallback to mock. */
function useResourceTotals() {
  const nullcloud = useNullCloudOptional();
  const mockMem = getMockMemory();
  const mockDisk = getMockDisk();
  const mockMemRatio = mockMem.totalGib > 0 ? mockMem.usedGib / mockMem.totalGib : 0.25;
  const mockDiskTotal = mockDisk.usedGib + mockDisk.freeGib;
  const mockDiskUsedRatio = mockDiskTotal > 0 ? mockDisk.usedGib / mockDiskTotal : 0.12;

  if (nullcloud) {
    const { localMachine } = nullcloud;
    const usedGib = localMachine.ramGib * mockMemRatio;
    const diskUsed = localMachine.diskTotalGib * mockDiskUsedRatio;
    const diskFree = localMachine.diskTotalGib - diskUsed;
    return {
      memory: { usedGib, totalGib: localMachine.ramGib },
      disk: { usedGib: diskUsed, freeGib: diskFree },
      diskTotal: localMachine.diskTotalGib,
    };
  }
  return {
    memory: mockMem,
    disk: mockDisk,
    diskTotal: mockDiskTotal,
  };
}

export default function SystemMonitorApp() {
  const [section, setSection] = useState<Section>("resources");
  const resourceTotals = useResourceTotals();
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

  const processes = useMemo(() => [...MOCK_PROCESSES].sort((a, b) => b.cpuPercent - a.cpuPercent), []);

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
                    {disk.usedGib} GiB used, {disk.freeGib} GiB free
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
            <p className={styles.mainSubtitle}>Running processes (mock data).</p>
            <div className={styles.processHeader}>
              <span className={styles.processHeaderName}>Name</span>
              <span className={styles.processHeaderPid}>PID</span>
              <span className={styles.processHeaderCpu}>CPU %</span>
              <span className={styles.processHeaderMemory}>Memory</span>
            </div>
            <ul className={styles.processList}>
              {processes.map((proc: MockProcess) => (
                <li key={proc.id} className={styles.processRow}>
                  <span className={styles.processName}>{proc.name}</span>
                  <span className={styles.processPid}>{proc.pid}</span>
                  <span className={styles.processCpu}>{proc.cpuPercent.toFixed(1)}</span>
                  <span className={styles.processMemory}>{proc.memoryMb} MiB</span>
                </li>
              ))}
            </ul>
          </>
        )}
      </main>
    </div>
  );
}
