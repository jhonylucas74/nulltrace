import { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { Settings, Bell } from "lucide-react";
import { useAuth } from "../contexts/AuthContext";
import { useNotification } from "../contexts/NotificationContext";
import { useWindowManager } from "../contexts/WindowManagerContext";
import { useWorkspaceLayout } from "../contexts/WorkspaceLayoutContext";
import styles from "./TopBar.module.css";

export default function TopBar() {
  const [time, setTime] = useState(() => formatTime(new Date()));
  const [userMenuOpen, setUserMenuOpen] = useState(false);
  const userMenuRef = useRef<HTMLDivElement>(null);
  const { username, logout } = useAuth();
  const navigate = useNavigate();
  const { setFocus, getWindowIdsByType } = useWindowManager();
  const { openApp } = useWorkspaceLayout();
  const { unreadCount, isDrawerOpen, openDrawer, closeDrawer } = useNotification();

  useEffect(() => {
    const id = setInterval(() => setTime(formatTime(new Date())), 1000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (userMenuRef.current && !userMenuRef.current.contains(e.target as Node)) {
        setUserMenuOpen(false);
      }
    }
    if (userMenuOpen) {
      document.addEventListener("mousedown", handleClickOutside);
      return () => document.removeEventListener("mousedown", handleClickOutside);
    }
  }, [userMenuOpen]);

  function handleEndSession() {
    setUserMenuOpen(false);
    logout();
    navigate("/login", { replace: true });
  }

  function handleSoundClick() {
    const ids = getWindowIdsByType("sound");
    if (ids.length > 0) {
      setFocus(ids[ids.length - 1]);
    } else {
      openApp("sound", { title: "Sound" });
    }
  }

  function handleWifiClick() {
    const ids = getWindowIdsByType("network");
    if (ids.length > 0) {
      setFocus(ids[ids.length - 1]);
    } else {
      openApp("network", { title: "Network" });
    }
  }

  function handleSettingsClick() {
    const ids = getWindowIdsByType("settings");
    if (ids.length > 0) {
      setFocus(ids[ids.length - 1]);
    } else {
      openApp("settings", { title: "Settings" });
    }
  }

  function handleNotificationClick() {
    if (isDrawerOpen) closeDrawer();
    else openDrawer();
  }

  return (
    <header className={styles.bar}>
      <div className={styles.left}>
        <span className={styles.logo}>
          <span className={styles.logoWordmarkNull}>null</span>
          <span className={styles.logoWordmarkTrace}>trace</span>
        </span>
      </div>
      <div className={styles.right}>
        <button
          type="button"
          className={styles.notificationBtn}
          onClick={handleNotificationClick}
          title="Notifications"
          aria-label="Notifications"
          aria-expanded={isDrawerOpen}
          aria-haspopup="true"
        >
          <Bell size={18} />
          {unreadCount > 0 && (
            <span className={styles.notificationBadge}>
              {unreadCount > 9 ? "9+" : unreadCount}
            </span>
          )}
        </button>
        <button
          type="button"
          className={styles.iconBtn}
          onClick={handleWifiClick}
          title="Network"
          aria-label="Network"
        >
          <WifiIcon />
        </button>
        <button
          type="button"
          className={styles.iconBtn}
          onClick={handleSoundClick}
          title="Sound"
          aria-label="Sound"
        >
          <SoundIcon />
        </button>
        <span className={styles.clock}>{time}</span>
        <button
          type="button"
          className={styles.iconBtn}
          onClick={handleSettingsClick}
          title="Settings"
          aria-label="Settings"
        >
          <Settings size={18} />
        </button>
        <div className={styles.userMenuWrap} ref={userMenuRef}>
          <button
            type="button"
            className={styles.userBtn}
            onClick={() => setUserMenuOpen((o) => !o)}
            title={username ?? "User"}
            aria-expanded={userMenuOpen}
            aria-haspopup="true"
          >
            <UserIcon />
          </button>
          {userMenuOpen && (
            <div className={styles.userDropdown}>
              <div className={styles.userDropdownHeader}>
                <UserIcon />
                <span>{username ?? "User"}</span>
              </div>
              <button type="button" className={styles.userDropdownItem} onClick={handleEndSession}>
                End session
              </button>
            </div>
          )}
        </div>
      </div>
    </header>
  );
}

function formatTime(d: Date): string {
  return d.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function WifiIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M5 12.55a11 11 0 0 1 14.08 0" />
      <path d="M1.42 9a16 16 0 0 1 21.16 0" />
      <path d="M8.53 16.11a6 6 0 0 1 6.95 0" />
      <line x1="12" y1="20" x2="12.01" y2="20" />
    </svg>
  );
}

function SoundIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" />
      <path d="M19.07 4.93a10 10 0 0 1 0 14.14" />
      <path d="M15.54 8.46a5 5 0 0 1 0 7.07" />
    </svg>
  );
}

function UserIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
      <circle cx="12" cy="7" r="4" />
    </svg>
  );
}
