import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTheme } from "../contexts/ThemeContext";
import type { ThemeId } from "../contexts/ThemeContext";
import { useAuth } from "../contexts/AuthContext";
import { NULLTRACE_ASCII_ART, NULLTRACE_ASCII_ART_INVERTED } from "../lib/nulltraceAsciiArt";
import styles from "./SysinfoApp.module.css";

const THEME_DISPLAY_NAMES: Record<ThemeId, string> = {
  latte: "Latte",
  frappe: "Frappé",
  macchiato: "Macchiato",
  mocha: "Mocha",
  onedark: "One Dark",
  dracula: "Dracula",
  githubdark: "Nulltrace",
  monokai: "Monokai",
  solardark: "Solarized Dark",
};

const MOCK_VERSION = "0.1.0";

interface SysinfoData {
  cpu_cores: number;
  memory_mb: number;
  disk_mb: number;
}

function formatRamTotalMb(mb: number): string {
  const gib = mb / 1024;
  return `${gib.toFixed(1)} GiB`;
}

function formatDiskTotalMb(mb: number): string {
  const gib = mb / 1024;
  return `${gib.toFixed(1)} GiB`;
}

export default function SysinfoApp() {
  const { theme } = useTheme();
  const { username, playerId, token } = useAuth();
  const [sysinfo, setSysinfo] = useState<SysinfoData | null>(null);
  const [, setSysinfoError] = useState<string | null>(null);
  const [profile, setProfile] = useState<{ rank: number; faction_name: string } | null>(null);

  const fetchSysinfo = useCallback(async () => {
    if (!playerId || !token) {
      setSysinfo(null);
      setSysinfoError(null);
      return;
    }
    try {
      const res = await invoke<{ cpu_cores: number; memory_mb: number; disk_mb: number; error_message: string }>(
        "grpc_sysinfo",
        { token }
      );
      if (res.error_message) {
        setSysinfoError(res.error_message);
        setSysinfo(null);
      } else {
        setSysinfo({ cpu_cores: res.cpu_cores, memory_mb: res.memory_mb, disk_mb: res.disk_mb });
        setSysinfoError(null);
      }
    } catch (e) {
      setSysinfoError(e instanceof Error ? e.message : String(e));
      setSysinfo(null);
    }
  }, [playerId, token]);

  const fetchProfile = useCallback(async () => {
    if (!token) {
      setProfile(null);
      return;
    }
    try {
      const res = await invoke<{ rank: number; faction_name: string; error_message: string }>(
        "grpc_get_player_profile",
        { token }
      );
      if (res.error_message) {
        setProfile(null);
      } else {
        setProfile({ rank: res.rank, faction_name: res.faction_name || "" });
      }
    } catch {
      setProfile(null);
    }
  }, [token]);

  useEffect(() => {
    fetchSysinfo();
  }, [fetchSysinfo]);

  useEffect(() => {
    fetchProfile();
  }, [fetchProfile]);

  const cpuStr = sysinfo != null ? `${sysinfo.cpu_cores} cores` : "—";
  const memoryStr = sysinfo != null ? formatRamTotalMb(sysinfo.memory_mb) : "—";
  const diskStr = sysinfo != null ? formatDiskTotalMb(sysinfo.disk_mb) : "—";
  const rankStr = profile != null && profile.rank > 0 ? `#${profile.rank}` : "—";
  const factionStr = profile?.faction_name ?? "—";

  return (
    <div className={styles.app}>
      <div className={styles.asciiColumn}>
        <div className={styles.asciiArtWrapper}>
          <pre className={styles.asciiArtInverted} aria-hidden>{NULLTRACE_ASCII_ART_INVERTED}</pre>
          <pre className={styles.asciiArt}>{NULLTRACE_ASCII_ART}</pre>
        </div>
      </div>
      <div className={styles.infoColumn}>
        <div className={styles.infoLine}><span className={styles.label}>OS</span> nulltrace</div>
        <div className={styles.infoLine}><span className={styles.label}>Version</span> {MOCK_VERSION}</div>
        <div className={styles.infoLine}><span className={styles.label}>Theme</span> {THEME_DISPLAY_NAMES[theme]}</div>
        <div className={styles.infoLine}><span className={styles.label}>CPU</span> {cpuStr}</div>
        <div className={styles.infoLine}><span className={styles.label}>Memory</span> {memoryStr}</div>
        <div className={styles.infoLine}><span className={styles.label}>Disk</span> {diskStr}</div>
        {username && <div className={styles.infoLine}><span className={styles.label}>User</span> {username}</div>}
        <div className={styles.infoLine}><span className={styles.label}>Rank</span> {rankStr}</div>
        <div className={styles.infoLine}><span className={styles.label}>Faction</span> {factionStr}</div>
      </div>
    </div>
  );
}
