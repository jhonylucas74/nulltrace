import { useEffect } from "react";
import { X } from "lucide-react";
import { useNotification } from "../contexts/NotificationContext";
import type { NotificationItem } from "../contexts/NotificationContext";
import styles from "./NotificationDrawer.module.css";

function formatRelativeTime(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins} min ago`;
  if (diffHours < 24) return `${diffHours} hour${diffHours !== 1 ? "s" : ""} ago`;
  if (diffDays < 7) return `${diffDays} day${diffDays !== 1 ? "s" : ""} ago`;
  return date.toLocaleDateString();
}

export default function NotificationDrawer() {
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
      aria-label="Notifications"
      onClick={(e) => e.target === e.currentTarget && closeDrawer()}
    >
      <div className={styles.panel} onClick={(e) => e.stopPropagation()}>
        <div className={styles.header}>
          <h2 className={styles.title}>Notifications</h2>
        </div>
        <div className={styles.list}>
          {notifications.length === 0 ? (
            <p className={styles.empty}>No notifications</p>
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
                  title="Remove notification"
                  aria-label={`Remove ${item.title} notification`}
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
            Clear all
          </button>
        </div>
      </div>
    </div>
  );
}
