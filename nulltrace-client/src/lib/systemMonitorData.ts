/**
 * Mock system monitor data (CPU, memory, disk, processes). No GPU.
 * Browser has no access to real system stats; all values are fictional.
 */

/** Mock CPU usage (0–100%). */
export function getMockCpuPercent(): number {
  return 24;
}

/** Mock memory: used GiB, total GiB. */
export function getMockMemory(): { usedGib: number; totalGib: number } {
  return { usedGib: 4.2, totalGib: 15.8 };
}

/** Mock disk: used GiB, free GiB. */
export function getMockDisk(): { usedGib: number; freeGib: number } {
  return { usedGib: 12, freeGib: 88 };
}

export interface MockProcess {
  id: string;
  name: string;
  pid: number;
  cpuPercent: number;
  memoryMb: number;
  user?: string;
}

/** Mock running processes (fictional names, no real brands). */
export const MOCK_PROCESSES: MockProcess[] = [
  { id: "p1", name: "nulltrace", pid: 1201, cpuPercent: 2.4, memoryMb: 320, user: "user" },
  { id: "p2", name: "ide", pid: 1198, cpuPercent: 1.8, memoryMb: 512, user: "user" },
  { id: "p3", name: "editor", pid: 1150, cpuPercent: 0.5, memoryMb: 180, user: "user" },
  { id: "p4", name: "dev-server", pid: 1180, cpuPercent: 0.2, memoryMb: 96, user: "user" },
  { id: "p5", name: "file-manager", pid: 1102, cpuPercent: 0.1, memoryMb: 145, user: "user" },
  { id: "p6", name: "desktop-shell", pid: 892, cpuPercent: 1.2, memoryMb: 420, user: "user" },
  { id: "p7", name: "runtime", pid: 1175, cpuPercent: 0.3, memoryMb: 88, user: "user" },
  { id: "p8", name: "audio-srv", pid: 756, cpuPercent: 0.0, memoryMb: 52, user: "user" },
  { id: "p9", name: "init", pid: 1, cpuPercent: 0.0, memoryMb: 128, user: "root" },
  { id: "p10", name: "bus-daemon", pid: 234, cpuPercent: 0.0, memoryMb: 24, user: "root" },
];
