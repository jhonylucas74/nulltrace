import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { useTheme } from "../contexts/ThemeContext";
import { useAuth } from "../contexts/AuthContext";
import { NULLTRACE_ASCII_ART, NULLTRACE_ASCII_ART_INVERTED } from "../lib/nulltraceAsciiArt";
import styles from "./SysinfoApp.module.css";

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
  const { t } = useTranslation("sysinfo");
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

  const cpuStr = sysinfo != null ? `${sysinfo.cpu_cores} ${t("cores")}` : "—";
  const memoryStr = sysinfo != null ? formatRamTotalMb(sysinfo.memory_mb) : "—";
  const diskStr = sysinfo != null ? formatDiskTotalMb(sysinfo.disk_mb) : "—";
  const rankStr = profile != null && profile.rank > 0 ? `#${profile.rank}` : "—";
  const themeName = t(`theme_${theme}`);
  const hasFaction = Boolean(profile?.faction_name?.trim());

  return (
    <div className={styles.app}>
      <div className={styles.asciiColumn}>
        <div className={styles.asciiArtWrapper}>
          <pre className={styles.asciiArtInverted} aria-hidden>{NULLTRACE_ASCII_ART_INVERTED}</pre>
          <pre className={styles.asciiArt}>{NULLTRACE_ASCII_ART}</pre>
        </div>
      </div>
      <div className={styles.infoColumn}>
        <div className={styles.infoLine}><span className={styles.label}>{t("label_os")}</span> nulltrace</div>
        <div className={styles.infoLine}><span className={styles.label}>{t("label_version")}</span> {MOCK_VERSION}</div>
        <div className={styles.infoLine}><span className={styles.label}>{t("label_theme")}</span> {themeName}</div>
        <div className={styles.infoLine}><span className={styles.label}>{t("label_cpu")}</span> {cpuStr}</div>
        <div className={styles.infoLine}><span className={styles.label}>{t("label_memory")}</span> {memoryStr}</div>
        <div className={styles.infoLine}><span className={styles.label}>{t("label_disk")}</span> {diskStr}</div>
        {username && <div className={styles.infoLine}><span className={styles.label}>{t("label_user")}</span> {username}</div>}
        <div className={styles.infoLine}><span className={styles.label}>{t("label_rank")}</span> {rankStr}</div>
        {hasFaction && (
          <div className={styles.infoLine}><span className={styles.label}>{t("label_faction")}</span> {profile!.faction_name}</div>
        )}
      </div>
    </div>
  );
}
