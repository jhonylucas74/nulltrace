/**
 * NullCloud: catalog and types for local machine upgrades and VPS plans.
 * All prices in USD. Local upgrades are one-time; VPS and Internet are billed weekly.
 */

/** Internet plan (weekly subscription) for the local machine. Affects download speed. */
export interface InternetPlan {
  id: string;
  name: string;
  speedMbps: number;
  weeklyPriceUsd: number;
  badge?: string;
}

/** Available internet plans. Basic is free; higher tiers are faster. */
export const INTERNET_PLANS: InternetPlan[] = [
  { id: "basic", name: "Basic", speedMbps: 5, weeklyPriceUsd: 0 },
  { id: "standard", name: "Standard", speedMbps: 50, weeklyPriceUsd: 3, badge: "Popular" },
  { id: "fast", name: "Fast", speedMbps: 100, weeklyPriceUsd: 5.5 },
  { id: "premium", name: "Premium", speedMbps: 500, weeklyPriceUsd: 11 },
];

export function getInternetPlanById(planId: string): InternetPlan | undefined {
  return INTERNET_PLANS.find((p) => p.id === planId);
}

/** Current spec of the player's local (virtual) machine. */
export interface LocalMachineSpec {
  cpuCores: number;
  ramGib: number;
  diskTotalGib: number;
  /** Current internet plan id. Default "basic". */
  internetPlanId?: string;
  /** Next billing date (Unix ms) for internet subscription. */
  internetPlanNextBilling?: number;
}

/** Default machine when the player has not upgraded. */
export const DEFAULT_LOCAL_MACHINE: LocalMachineSpec = {
  cpuCores: 2,
  ramGib: 8,
  diskTotalGib: 100,
  internetPlanId: "basic",
  internetPlanNextBilling: undefined,
};

/** Single upgrade option for one dimension (CPU, RAM, or Disk). */
export interface LocalUpgradeOption {
  value: number;
  label: string;
  priceUsd: number;
}

/** Catalog: CPU tiers (one-time purchase). */
export const CPU_UPGRADES: LocalUpgradeOption[] = [
  { value: 2, label: "2 cores", priceUsd: 0 },
  { value: 4, label: "4 cores", priceUsd: 49 },
  { value: 6, label: "6 cores", priceUsd: 99 },
  { value: 8, label: "8 cores", priceUsd: 149 },
  { value: 10, label: "10 cores", priceUsd: 199 },
  { value: 12, label: "12 cores", priceUsd: 249 },
  { value: 16, label: "16 cores", priceUsd: 329 },
  { value: 24, label: "24 cores", priceUsd: 449 },
];

/** Catalog: RAM tiers (one-time purchase). */
export const RAM_UPGRADES: LocalUpgradeOption[] = [
  { value: 8, label: "8 GiB", priceUsd: 0 },
  { value: 16, label: "16 GiB", priceUsd: 39 },
  { value: 32, label: "32 GiB", priceUsd: 89 },
  { value: 64, label: "64 GiB", priceUsd: 179 },
  { value: 96, label: "96 GiB", priceUsd: 249 },
  { value: 128, label: "128 GiB", priceUsd: 319 },
  { value: 192, label: "192 GiB", priceUsd: 449 },
  { value: 256, label: "256 GiB", priceUsd: 579 },
];

/** Catalog: Disk tiers (one-time purchase). */
export const DISK_UPGRADES: LocalUpgradeOption[] = [
  { value: 100, label: "100 GiB", priceUsd: 0 },
  { value: 250, label: "250 GiB", priceUsd: 29 },
  { value: 500, label: "500 GiB", priceUsd: 59 },
  { value: 1000, label: "1 TiB", priceUsd: 99 },
  { value: 1500, label: "1.5 TiB", priceUsd: 139 },
  { value: 2000, label: "2 TiB", priceUsd: 179 },
  { value: 3000, label: "3 TiB", priceUsd: 249 },
  { value: 4000, label: "4 TiB", priceUsd: 319 },
];

/** VPS plan (weekly subscription). Instance types inspired by cloud provider tiers. */
export interface VpsPlan {
  id: string;
  name: string;
  /** Instance type label (e.g. "nano", "micro") for display. */
  tier: string;
  cpuCores: number;
  ramGib: number;
  diskGib: number;
  /** Price per week in USD. */
  weeklyPriceUsd: number;
  /** Optional badge: "Popular", "Best value", etc. */
  badge?: string;
}

/** Available VPS plans (broad range of instance sizes). Billed weekly. */
export const VPS_PLANS: VpsPlan[] = [
  { id: "nano", name: "Nano", tier: "nano", cpuCores: 1, ramGib: 0.5, diskGib: 10, weeklyPriceUsd: 1 },
  { id: "micro", name: "Micro", tier: "micro", cpuCores: 1, ramGib: 1, diskGib: 20, weeklyPriceUsd: 2 },
  { id: "small", name: "Small", tier: "small", cpuCores: 1, ramGib: 2, diskGib: 40, weeklyPriceUsd: 3, badge: "Popular" },
  { id: "medium", name: "Medium", tier: "medium", cpuCores: 2, ramGib: 4, diskGib: 80, weeklyPriceUsd: 5 },
  { id: "large", name: "Large", tier: "large", cpuCores: 4, ramGib: 8, diskGib: 160, weeklyPriceUsd: 11 },
  { id: "xlarge", name: "X-Large", tier: "xlarge", cpuCores: 8, ramGib: 16, diskGib: 320, weeklyPriceUsd: 22 },
  { id: "2xlarge", name: "2X-Large", tier: "2xlarge", cpuCores: 16, ramGib: 32, diskGib: 640, weeklyPriceUsd: 42, badge: "Best value" },
  /* Legacy IDs for existing saves */
  { id: "starter", name: "Starter", tier: "small", cpuCores: 1, ramGib: 2, diskGib: 20, weeklyPriceUsd: 2 },
  { id: "pro", name: "Pro", tier: "medium", cpuCores: 2, ramGib: 4, diskGib: 50, weeklyPriceUsd: 5 },
  { id: "max", name: "Max", tier: "large", cpuCores: 4, ramGib: 8, diskGib: 100, weeklyPriceUsd: 10 },
];

/** A VPS instance owned by the player. */
export interface VpsInstance {
  id: string;
  planId: string;
  ip: string;
  sshUser: string;
  sshPassword: string;
  createdAt: number;
  nextBillingDate: number;
}

/** Generate a fake private IP for a new VPS (in-game only). */
export function generateVpsIp(): string {
  return `10.${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}.${Math.floor(1 + Math.random() * 254)}`;
}

/** Next billing date 7 days from now (weekly billing). */
export function nextBillingIn7Days(): number {
  const d = new Date();
  d.setDate(d.getDate() + 7);
  return d.getTime();
}

let nextVpsId = 1;
export function nextVpsInstanceId(): string {
  return `vps-${nextVpsId++}`;
}

export function getPlanById(planId: string): VpsPlan | undefined {
  return VPS_PLANS.find((p) => p.id === planId);
}
