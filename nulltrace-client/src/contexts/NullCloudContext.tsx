/**
 * NullCloud state: local machine spec, VPS list, upgrade/buy/setCredentials.
 * Persists to localStorage. Must be used within WalletProvider (uses pay()).
 */

import React, { createContext, useContext, useCallback, useMemo, useState, useEffect } from "react";
import { useWallet } from "./WalletContext";
import {
  DEFAULT_LOCAL_MACHINE,
  type LocalMachineSpec,
  type VpsInstance,
  type VpsPlan,
  type InternetPlan,
  VPS_PLANS,
  INTERNET_PLANS,
  getPlanById,
  getInternetPlanById,
  generateVpsIp,
  nextBillingIn7Days,
  getNextVpsInstanceId,
  CPU_UPGRADES,
  RAM_UPGRADES,
  DISK_UPGRADES,
} from "../lib/nullcloudData";

const STORAGE_KEY_MACHINE = "nullcloud-local-machine";
const STORAGE_KEY_VPS = "nullcloud-vps-list";

function loadLocalMachine(): LocalMachineSpec {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_MACHINE);
    if (!raw) return DEFAULT_LOCAL_MACHINE;
    const parsed = JSON.parse(raw) as LocalMachineSpec;
    if (
      typeof parsed.cpuCores === "number" &&
      typeof parsed.ramGib === "number" &&
      typeof parsed.diskTotalGib === "number"
    ) {
      return {
        ...DEFAULT_LOCAL_MACHINE,
        ...parsed,
        cpuCores: parsed.cpuCores,
        ramGib: parsed.ramGib,
        diskTotalGib: parsed.diskTotalGib,
      };
    }
  } catch {
    // ignore
  }
  return DEFAULT_LOCAL_MACHINE;
}

function loadVpsList(): VpsInstance[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_VPS);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as VpsInstance[];
    if (Array.isArray(parsed)) return parsed;
  } catch {
    // ignore
  }
  return [];
}

export interface NullCloudContextValue {
  localMachine: LocalMachineSpec;
  vpsList: VpsInstance[];
  /** One-time upgrade for local machine. Returns true if payment succeeded and spec was updated. */
  upgradeLocal: (
    upgrade: { cpuCores?: number; ramGib?: number; diskTotalGib?: number },
    costUsd: number
  ) => boolean;
  /** Buy first week of a VPS plan. Returns the new VPS id if successful, null otherwise. */
  buyVps: (planId: string) => string | null;
  /** Upgrade a VPS to a new plan (pay new plan first week; advance billing). */
  upgradeVps: (vpsId: string, newPlanId: string) => boolean;
  /** Set SSH user and password for a VPS (no payment). */
  setVpsCredentials: (vpsId: string, user: string, password: string) => void;
  /** Subscribe to an internet plan (weekly). Pay first week; sets plan and next billing. */
  subscribeInternet: (planId: string) => boolean;
  /** Current download speed in Mbps from the active internet plan. */
  getDownloadSpeedMbps: () => number;
  /** Catalog for UI. */
  cpuUpgrades: typeof CPU_UPGRADES;
  ramUpgrades: typeof RAM_UPGRADES;
  diskUpgrades: typeof DISK_UPGRADES;
  vpsPlans: VpsPlan[];
  internetPlans: InternetPlan[];
  getPlanById: (planId: string) => VpsPlan | undefined;
  getInternetPlanById: (planId: string) => InternetPlan | undefined;
}

const NullCloudContext = createContext<NullCloudContextValue | null>(null);

export function NullCloudProvider({ children }: { children: React.ReactNode }) {
  const wallet = useWallet();
  const [localMachine, setLocalMachine] = useState<LocalMachineSpec>(loadLocalMachine);
  const [vpsList, setVpsList] = useState<VpsInstance[]>(loadVpsList);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY_MACHINE, JSON.stringify(localMachine));
  }, [localMachine]);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY_VPS, JSON.stringify(vpsList));
  }, [vpsList]);

  const upgradeLocal = useCallback(
    (upgrade: { cpuCores?: number; ramGib?: number; diskTotalGib?: number }, costUsd: number): boolean => {
      if (costUsd > 0 && !wallet.pay("USD", costUsd, "NullCloud: machine upgrade")) return false;
      setLocalMachine((prev) => ({
        cpuCores: upgrade.cpuCores ?? prev.cpuCores,
        ramGib: upgrade.ramGib ?? prev.ramGib,
        diskTotalGib: upgrade.diskTotalGib ?? prev.diskTotalGib,
      }));
      return true;
    },
    [wallet]
  );

  const buyVps = useCallback(
    (planId: string): string | null => {
      const plan = getPlanById(planId);
      if (!plan) return null;
      if (!wallet.pay("USD", plan.weeklyPriceUsd, `NullCloud: VPS ${plan.name} (weekly)`)) return null;
      let createdId: string;
      setVpsList((prev) => {
        createdId = getNextVpsInstanceId(prev.map((v) => v.id));
        const vps: VpsInstance = {
          id: createdId,
          planId: plan.id,
          ip: generateVpsIp(),
          sshUser: "",
          sshPassword: "",
          createdAt: Date.now(),
          nextBillingDate: nextBillingIn7Days(),
        };
        return [...prev, vps];
      });
      return createdId!;
    },
    [wallet]
  );

  const upgradeVps = useCallback(
    (vpsId: string, newPlanId: string): boolean => {
      const plan = getPlanById(newPlanId);
      if (!plan) return false;
      const vps = vpsList.find((v) => v.id === vpsId);
      if (!vps) return false;
      if (!wallet.pay("USD", plan.weeklyPriceUsd, `NullCloud: VPS upgrade to ${plan.name} (weekly)`)) return false;
      setVpsList((prev) =>
        prev.map((v) =>
          v.id === vpsId
            ? { ...v, planId: plan.id, nextBillingDate: nextBillingIn7Days() }
            : v
        )
      );
      return true;
    },
    [wallet, vpsList]
  );

  const setVpsCredentials = useCallback((vpsId: string, user: string, password: string) => {
    setVpsList((prev) =>
      prev.map((v) => (v.id === vpsId ? { ...v, sshUser: user, sshPassword: password } : v))
    );
  }, []);

  const subscribeInternet = useCallback(
    (planId: string): boolean => {
      const plan = getInternetPlanById(planId);
      if (!plan) return false;
      if (plan.weeklyPriceUsd > 0 && !wallet.pay("USD", plan.weeklyPriceUsd, `NullCloud: Internet ${plan.name} (weekly)`))
        return false;
      setLocalMachine((prev) => ({
        ...prev,
        internetPlanId: planId,
        internetPlanNextBilling: nextBillingIn7Days(),
      }));
      return true;
    },
    [wallet]
  );

  const getDownloadSpeedMbps = useCallback((): number => {
    const planId = localMachine.internetPlanId ?? "basic";
    const plan = getInternetPlanById(planId);
    return plan?.speedMbps ?? 0;
  }, [localMachine.internetPlanId]);

  const value: NullCloudContextValue = useMemo(
    () => ({
      localMachine,
      vpsList,
      upgradeLocal,
      buyVps,
      upgradeVps,
      setVpsCredentials,
      subscribeInternet,
      getDownloadSpeedMbps,
      cpuUpgrades: CPU_UPGRADES,
      ramUpgrades: RAM_UPGRADES,
      diskUpgrades: DISK_UPGRADES,
      vpsPlans: VPS_PLANS,
      internetPlans: INTERNET_PLANS,
      getPlanById,
      getInternetPlanById,
    }),
    [
      localMachine,
      vpsList,
      upgradeLocal,
      buyVps,
      upgradeVps,
      setVpsCredentials,
      subscribeInternet,
      getDownloadSpeedMbps,
    ]
  );

  return (
    <NullCloudContext.Provider value={value}>
      {children}
    </NullCloudContext.Provider>
  );
}

export function useNullCloud(): NullCloudContextValue {
  const ctx = useContext(NullCloudContext);
  if (!ctx) throw new Error("useNullCloud must be used within NullCloudProvider");
  return ctx;
}

/** Optional hook: returns null if not inside NullCloudProvider (e.g. for System Monitor). */
export function useNullCloudOptional(): NullCloudContextValue | null {
  return useContext(NullCloudContext);
}
