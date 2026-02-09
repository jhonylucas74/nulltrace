import React, { createContext, useContext, useState, useCallback, useMemo } from "react";

export interface NotificationItem {
  id: string;
  title: string;
  body?: string;
  read: boolean;
  date: Date;
}

const MOCK_NOTIFICATIONS: NotificationItem[] = [
  { id: "1", title: "System", body: "Welcome to Nulltrace.", read: false, date: new Date() },
  {
    id: "2",
    title: "Desktop",
    body: "Grid layout is available. Enable it from Startup settings.",
    read: false,
    date: new Date(Date.now() - 3600000),
  },
  {
    id: "3",
    title: "Wallet",
    body: "Your balance has been updated.",
    read: true,
    date: new Date(Date.now() - 7200000),
  },
  {
    id: "4",
    title: "NullCloud",
    body: "New VPS plans are available.",
    read: false,
    date: new Date(Date.now() - 86400000),
  },
  {
    id: "5",
    title: "Hackerboard",
    body: "You moved up in the rankings.",
    read: true,
    date: new Date(Date.now() - 172800000),
  },
];

interface NotificationContextValue {
  notifications: NotificationItem[];
  unreadCount: number;
  clearAll: () => void;
  removeNotification: (id: string) => void;
  isDrawerOpen: boolean;
  openDrawer: () => void;
  closeDrawer: () => void;
}

const NotificationContext = createContext<NotificationContextValue | null>(null);

export function NotificationProvider({ children }: { children: React.ReactNode }) {
  const [notifications, setNotifications] = useState<NotificationItem[]>(() => MOCK_NOTIFICATIONS);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);

  const unreadCount = useMemo(
    () => notifications.filter((n) => !n.read).length,
    [notifications]
  );

  const clearAll = useCallback(() => {
    setNotifications([]);
  }, []);

  const removeNotification = useCallback((id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  }, []);

  const openDrawer = useCallback(() => setIsDrawerOpen(true), []);
  const closeDrawer = useCallback(() => setIsDrawerOpen(false), []);

  const value: NotificationContextValue = {
    notifications,
    unreadCount,
    clearAll,
    removeNotification,
    isDrawerOpen,
    openDrawer,
    closeDrawer,
  };

  return (
    <NotificationContext.Provider value={value}>
      {children}
    </NotificationContext.Provider>
  );
}

export function useNotification(): NotificationContextValue {
  const ctx = useContext(NotificationContext);
  if (!ctx) throw new Error("useNotification must be used within NotificationProvider");
  return ctx;
}
