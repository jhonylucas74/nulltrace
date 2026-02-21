import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";
import { useNotification } from "../contexts/NotificationContext";
import type { NotificationItem } from "../contexts/NotificationContext";
import styles from "./NotificationDrawer.module.css";

function useFormatRelativeTime() {
  const { t } = useTranslation("notifications");
  return (date: Date): string => {
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return t("just_now");
    if (diffMins < 60) return t("mins_ago", { count: diffMins });
    if (diffHours < 24) return t("hours_ago", { count: diffHours });
    if (diffDays < 7) return t("days_ago", { count: diffDays });
    return date.toLocaleDateString();
  };
}

export default function NotificationDrawer() {
  const { t } = useTranslation("notifications");
  const { t: tCommon } = useTranslation("common");
  const formatRelativeTime = useFormatRelativeTime();
  const { notifications, clearAll, removeNotification, closeDrawer } = useNotification();

  useEffect(() => {
    function handleEscape(e: KeyboardEvent) {
      if (e.key === "Escape") closeDrawer();
    }
    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [closeDrawer]);

  return (
    <div
      className={styles.overlay}
      role="dialog"
      aria-modal="true"
      aria-label={t("title")}
      onClick={(e) => e.target === e.currentTarget && closeDrawer()}
    >
      <div className={styles.panel} onClick={(e) => e.stopPropagation()}>
        <div className={styles.header}>
          <h2 className={styles.title}>{t("title")}</h2>
        </div>
        <div className={styles.list}>
          {notifications.length === 0 ? (
            <p className={styles.empty}>{t("empty")}</p>
          ) : (
            notifications.map((item: NotificationItem) => (
              <div key={item.id} className={styles.item}>
                <div className={styles.itemContent}>
                  <div className={styles.itemTitle}>{item.title}</div>
                  {item.body && <div className={styles.itemBody}>{item.body}</div>}
                  <div className={styles.itemTime}>{formatRelativeTime(item.date)}</div>
                </div>
                <button
                  type="button"
                  className={styles.itemClearBtn}
                  onClick={(e) => {
                    e.stopPropagation();
                    removeNotification(item.id);
                  }}
                  title={t("remove")}
                  aria-label={t("remove_named", { title: item.title })}
                >
                  <X size={16} />
                </button>
              </div>
            ))
          )}
        </div>
        <div className={styles.footer}>
          <button
            type="button"
            className={styles.clearBtn}
            onClick={clearAll}
            disabled={notifications.length === 0}
          >
            {tCommon("clear_all")}
          </button>
        </div>
      </div>
    </div>
  );
}
