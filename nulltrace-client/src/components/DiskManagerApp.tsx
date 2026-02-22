import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { HardDrive, RotateCcw } from "lucide-react";
import { useAuth } from "../contexts/AuthContext";
import styles from "./DiskManagerApp.module.css";

/** Format bytes to human-readable string (e.g. "12.5 MiB", "20 GiB"). */
function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const value = bytes / Math.pow(1024, i);
  return `${value % 1 === 0 ? value : value.toFixed(1)} ${units[i]}`;
}

export default function DiskManagerApp() {
  const { t } = useTranslation("diskmanager");
  const { t: tCommon } = useTranslation("common");
  const { playerId, token, logout } = useAuth();
  const navigate = useNavigate();
  const [usedBytes, setUsedBytes] = useState<number | null>(null);
  const [totalBytes, setTotalBytes] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [restoring, setRestoring] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);

  const fetchDiskUsage = useCallback(async () => {
    if (!playerId || !token) {
      setError(t("error_not_logged_in"));
      setLoading(false);
      return;
    }
    const tauri = (window as unknown as { __TAURI__?: unknown }).__TAURI__;
    if (!tauri) {
      setError(t("error_desktop_required"));
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const res = await invoke<{ used_bytes: number; total_bytes: number; error_message: string }>(
        "grpc_disk_usage",
        { token }
      );
      if (res.error_message) {
        if (res.error_message === "UNAUTHENTICATED") {
          logout();
          navigate("/login");
          return;
        }
        setError(res.error_message);
      } else {
        setUsedBytes(res.used_bytes);
        setTotalBytes(res.total_bytes);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [playerId, token, logout, navigate]);

  useEffect(() => {
    fetchDiskUsage();
  }, [fetchDiskUsage]);

  const handleRestore = useCallback(async () => {
    if (!playerId || !token) return;
    setConfirmOpen(false);
    setRestoring(true);
    setError(null);
    try {
      const res = await invoke<{ success: boolean; error_message: string }>("grpc_restore_disk", {
        token,
      });
      if (res.success) {
        logout();
        navigate("/login", { replace: true });
      } else {
        if (res.error_message === "UNAUTHENTICATED") {
          logout();
          navigate("/login");
          return;
        }
        setError(res.error_message || t("error_restore_failed"));
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
} finally {
        setRestoring(false);
    }
  }, [playerId, fetchDiskUsage, t]);

  const progressPercent =
    totalBytes != null && totalBytes > 0 && usedBytes != null
      ? Math.min(100, (usedBytes / totalBytes) * 100)
      : 0;

  return (
    <div className={styles.app}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarTitle}>{t("sidebar_title")}</div>
        <button type="button" className={`${styles.navItem} ${styles.navItemActive}`}>
          <span className={styles.navIcon}>
            <HardDrive size={18} />
          </span>
          {t("nav_storage")}
        </button>
        <button type="button" className={styles.navItem}>
          <span className={styles.navIcon}>
            <RotateCcw size={18} />
          </span>
          {t("nav_restore")}
        </button>
      </aside>
      <div className={styles.main}>
        <div className={styles.content}>
          <div className={styles.sectionHeader}>
            <h2 className={styles.sectionTitle}>{t("section_storage")}</h2>
          </div>
          <p className={styles.hint}>
            {t("hint_storage")}
          </p>
          <div className={styles.card}>
            {loading ? (
              <p className={styles.loading}>{t("loading")}</p>
            ) : error ? (
              <p className={styles.error}>{error}</p>
            ) : usedBytes != null && totalBytes != null ? (
              <>
                <div className={styles.storageRow}>
                  <span className={styles.storageLabel}>{t("label_used")}</span>
                  <span className={styles.storageValue}>
                    {formatBytes(usedBytes)} / {formatBytes(totalBytes)}
                  </span>
                </div>
                <div className={styles.progressWrap}>
                  <div
                    className={styles.progressBar}
                    role="progressbar"
                    aria-valuenow={progressPercent}
                    aria-valuemin={0}
                    aria-valuemax={100}
                    style={{ width: `${progressPercent}%` }}
                  />
                </div>
              </>
            ) : null}
          </div>

          <div className={styles.sectionHeader} style={{ marginTop: "1.5rem" }}>
            <h2 className={styles.sectionTitle}>{t("section_restore")}</h2>
          </div>
          <p className={styles.hint}>
            {t("hint_restore")}
          </p>
          <div className={styles.card}>
            <button
              type="button"
              className={styles.restoreBtn}
              disabled={loading || restoring || !playerId}
              onClick={() => setConfirmOpen(true)}
            >
              {restoring ? t("restoring") : t("restore_btn")}
            </button>
          </div>
        </div>
      </div>

      {confirmOpen && (
        <div className={styles.modalOverlay} onClick={() => setConfirmOpen(false)}>
          <div className={styles.modal} onClick={(e) => e.stopPropagation()}>
            <h3 className={styles.modalTitle}>{t("modal_title")}</h3>
            <p className={styles.modalText}>
              {t("modal_text")}
            </p>
            <div className={styles.modalActions}>
              <button
                type="button"
                className={styles.modalCancel}
                onClick={() => setConfirmOpen(false)}
              >
                {tCommon("cancel")}
              </button>
              <button
                type="button"
                className={styles.modalConfirm}
                onClick={handleRestore}
                disabled={restoring}
              >
                {restoring ? t("restoring") : t("modal_restore")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
