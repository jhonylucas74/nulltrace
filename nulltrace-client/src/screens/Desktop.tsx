import React, { useEffect, useRef, useState, useCallback } from "react";
import { Palette, Cpu, Keyboard, Activity, Cloud } from "lucide-react";
import { useAuth } from "../contexts/AuthContext";
import { WalletProvider } from "../contexts/WalletContext";
import { WindowManagerProvider, useWindowManager } from "../contexts/WindowManagerContext";
import { WorkspaceLayoutProvider, useWorkspaceLayout, getWorkspaceArea, getSlotBounds } from "../contexts/WorkspaceLayoutContext";
import { FilePickerProvider, useFilePicker, getDefaultInitialPath } from "../contexts/FilePickerContext";
import { AppLauncherProvider, useAppLauncher } from "../contexts/AppLauncherContext";
import { ShortcutsProvider } from "../contexts/ShortcutsContext";
import type { WindowType } from "../contexts/WindowManagerContext";
import TopBar from "../components/TopBar";
import AppLauncher from "../components/AppLauncher";
import Dock from "../components/Dock";
import LayoutPanel from "../components/LayoutPanel";
import WorkspaceDots from "../components/WorkspaceDots";
import Window from "../components/Window";
import Terminal from "../components/Terminal";
import ThemeApp from "../components/ThemeApp";
import Explorer from "../components/Explorer";
import CodeEditor from "../components/CodeEditor";
import Browser from "../components/Browser";
import SoundManager from "../components/SoundManager";
import NetworkManager from "../components/NetworkManager";
import EmailApp from "../components/EmailApp";
import WalletApp from "../components/WalletApp";
import PixelArtApp from "../components/PixelArtApp";
import SysinfoApp from "../components/SysinfoApp";
import ShortcutsApp from "../components/ShortcutsApp";
import SystemMonitorApp from "../components/SystemMonitorApp";
import { NullCloudProvider } from "../contexts/NullCloudContext";
import { PaymentFeedbackProvider } from "../contexts/PaymentFeedbackContext";
import NullCloudApp from "../components/NullCloudApp";
import ShortcutsHandler from "../components/ShortcutsHandler";
import FilePicker from "../components/FilePicker";
import styles from "./Desktop.module.css";

function TerminalIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <polyline points="4 17 10 11 4 5" />
      <line x1="12" y1="19" x2="20" y2="19" />
    </svg>
  );
}

function ExplorerIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      <line x1="12" y1="11" x2="12" y2="17" />
      <line x1="9" y1="14" x2="15" y2="14" />
    </svg>
  );
}

function BrowserIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <circle cx="12" cy="12" r="10" />
      <line x1="2" y1="12" x2="22" y2="12" />
      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
    </svg>
  );
}

function AppsIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <rect x="3" y="3" width="7" height="7" rx="1" />
      <rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" />
      <rect x="14" y="14" width="7" height="7" rx="1" />
    </svg>
  );
}

function EditorIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <polyline points="16 18 22 12 16 6" />
      <polyline points="8 6 2 12 8 18" />
    </svg>
  );
}

function ThemeIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <path d="M12 2a10 10 0 0 0 0 20V2z" fill="currentColor" />
    </svg>
  );
}

function SoundIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" />
      <path d="M19.07 4.93a10 10 0 0 1 0 14.14" />
      <path d="M15.54 8.46a5 5 0 0 1 0 7.07" />
    </svg>
  );
}

function WifiIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M5 12.55a11 11 0 0 1 14.08 0" />
      <path d="M1.42 9a16 16 0 0 1 21.16 0" />
      <path d="M8.53 16.11a6 6 0 0 1 6.95 0" />
      <line x1="12" y1="20" x2="12.01" y2="20" />
    </svg>
  );
}

function MailIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" />
      <polyline points="22,6 12,13 2,6" />
    </svg>
  );
}

function WalletIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="1" y="4" width="22" height="16" rx="2" ry="2" />
      <line x1="1" y1="10" x2="23" y2="10" />
      <path d="M17 14a2 2 0 1 0 0-4 2 2 0 0 0 0 4z" />
    </svg>
  );
}

function PixelArtIcon() {
  return <Palette size={12} />;
}

function SysinfoIcon() {
  return <Cpu size={12} />;
}

function ShortcutsIcon() {
  return <Keyboard size={12} />;
}

function SysmonIcon() {
  return <Activity size={12} />;
}

function NullCloudIcon() {
  return <Cloud size={12} />;
}

const WINDOW_ICONS: Record<WindowType, React.ReactNode> = {
  terminal: <TerminalIcon />,
  explorer: <ExplorerIcon />,
  browser: <BrowserIcon />,
  apps: <AppsIcon />,
  editor: <EditorIcon />,
  theme: <ThemeIcon />,
  sound: <SoundIcon />,
  network: <WifiIcon />,
  email: <MailIcon />,
  wallet: <WalletIcon />,
  pixelart: <PixelArtIcon />,
  sysinfo: <SysinfoIcon />,
  shortcuts: <ShortcutsIcon />,
  sysmon: <SysmonIcon />,
  nullcloud: <NullCloudIcon />,
};

function PlaceholderContent({ title }: { title: string }) {
  return (
    <div className={styles.placeholder}>
      <p>{title}</p>
      <p className={styles.placeholderSub}>Coming soon</p>
    </div>
  );
}

function DesktopContent() {
  const { username } = useAuth();
  const { windows, focusedId, close, minimize, maximize, setFocus, move, resize, setWindowGridSlot } = useWindowManager();
  const {
    activeWorkspaceId,
    workspaces,
    openApp,
    gridModeEnabled,
    layoutPreset,
    getSlotAtPoint,
    getOccupiedSlots,
    getFirstFreeSlot,
    getLayoutForWorkspace,
    moveWindowToWorkspace,
  } = useWorkspaceLayout();
  const { isOpen: filePickerOpen, options: filePickerOptions, closeFilePicker } = useFilePicker();
  const { isOpen: appLauncherOpen } = useAppLauncher();
  const hasOpenedSysinfoRef = useRef(false);
  const [snapPreview, setSnapPreview] = useState<{ x: number; y: number; width: number; height: number } | null>(null);
  const leftBottomRef = useRef<HTMLDivElement>(null);
  const [draggingWindowId, setDraggingWindowId] = useState<string | null>(null);
  const [dragCursorX, setDragCursorX] = useState(0);
  const [dragCursorY, setDragCursorY] = useState(0);
  const [workspaceDropTarget, setWorkspaceDropTarget] = useState<{
    workspaceId: string;
    dotCenterX: number;
    dotCenterY: number;
  } | null>(null);

  const WORKSPACE_DROP_ZONE_THRESHOLD = 80;

  const firstWorkspaceId = workspaces[0]?.id ?? "";
  const visibleWindows = windows.filter(
    (w) =>
      !w.minimized &&
      w.type !== "apps" &&
      (w.workspaceId === activeWorkspaceId || (w.workspaceId === "" && activeWorkspaceId === firstWorkspaceId))
  );

  useEffect(() => {
    if (!username || hasOpenedSysinfoRef.current) return;
    hasOpenedSysinfoRef.current = true;
    openApp("sysinfo");
  }, [username, openApp]);

  useEffect(() => {
    if (!gridModeEnabled) setSnapPreview(null);
  }, [gridModeEnabled]);

  const handleDragStart = useCallback((id: string) => {
    setDraggingWindowId(id);
  }, []);

  const handleDragMove = useCallback(
    (_id: string, clientX: number, clientY: number) => {
      setDragCursorX(clientX);
      setDragCursorY(clientY);

      const container = leftBottomRef.current;
      if (!container) {
        setWorkspaceDropTarget(null);
        if (gridModeEnabled) {
          const area = getWorkspaceArea();
          const slot = getSlotAtPoint(area, clientX, clientY);
          if (slot) {
            const bounds = getSlotBounds(layoutPreset, slot, area);
            setSnapPreview({ x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height });
          } else {
            setSnapPreview(null);
          }
        }
        return;
      }

      const rect = container.getBoundingClientRect();
      const expanded = {
        left: rect.left - WORKSPACE_DROP_ZONE_THRESHOLD,
        right: rect.right + WORKSPACE_DROP_ZONE_THRESHOLD,
        top: rect.top - WORKSPACE_DROP_ZONE_THRESHOLD,
        bottom: rect.bottom + WORKSPACE_DROP_ZONE_THRESHOLD,
      };
      const inZone =
        clientX >= expanded.left &&
        clientX <= expanded.right &&
        clientY >= expanded.top &&
        clientY <= expanded.bottom;

      if (inZone) {
        setSnapPreview(null);
        const dots = container.querySelectorAll("[data-workspace-dot]");
        let nearest: { workspaceId: string; dotCenterX: number; dotCenterY: number } | null = null;
        let minDist = Infinity;
        for (let i = 0; i < dots.length; i++) {
          const el = dots[i] as HTMLElement;
          const r = el.getBoundingClientRect();
          const cx = r.left + r.width / 2;
          const cy = r.top + r.height / 2;
          const dist = (clientX - cx) ** 2 + (clientY - cy) ** 2;
          const wsId = el.getAttribute("data-workspace-id");
          if (wsId && dist < minDist) {
            minDist = dist;
            nearest = { workspaceId: wsId, dotCenterX: cx, dotCenterY: cy };
          }
        }
        setWorkspaceDropTarget(nearest);
      } else {
        setWorkspaceDropTarget(null);
        if (gridModeEnabled) {
          const area = getWorkspaceArea();
          const slot = getSlotAtPoint(area, clientX, clientY);
          if (slot) {
            const bounds = getSlotBounds(layoutPreset, slot, area);
            setSnapPreview({ x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height });
          } else {
            setSnapPreview(null);
          }
        }
      }
    },
    [gridModeEnabled, layoutPreset, getSlotAtPoint]
  );

  const handleDragEnd = useCallback(
    (id: string, lastX: number, lastY: number, centerClientX: number, centerClientY: number) => {
      setDraggingWindowId(null);

      if (workspaceDropTarget) {
        const { workspaceId } = workspaceDropTarget;
        const area = getWorkspaceArea();
        const slotResult = getFirstFreeSlot(workspaceId);
        const draggedWin = windows.find((w) => w.id === id);
        if (draggedWin) {
          if (slotResult) {
            const preset = getLayoutForWorkspace(workspaceId);
            const bounds = getSlotBounds(preset, slotResult.slot, area);
            move(id, bounds.x, bounds.y);
            resize(id, bounds.width, bounds.height);
            setWindowGridSlot(id, slotResult.slot);
          } else {
            const cx = area.left + area.width / 2 - draggedWin.size.width / 2;
            const cy = area.top + area.height / 2 - draggedWin.size.height / 2;
            move(id, cx, cy);
            setWindowGridSlot(id, undefined);
          }
          moveWindowToWorkspace(id, workspaceId);
        }
        setWorkspaceDropTarget(null);
        setSnapPreview(null);
        return;
      }

      setSnapPreview(null);
      if (!gridModeEnabled) return;
      const area = getWorkspaceArea();
      const slot = getSlotAtPoint(area, centerClientX, centerClientY);
      if (!slot) return;
      const bounds = getSlotBounds(layoutPreset, slot, area);
      const occupied = getOccupiedSlots(activeWorkspaceId, id);
      const slotKey = `${slot.row},${slot.col}`;
      const otherId = occupied.get(slotKey);
      const draggedWin = windows.find((w) => w.id === id);
      if (!draggedWin) return;

      if (otherId) {
        const otherWin = windows.find((w) => w.id === otherId);
        if (!otherWin) return;
        const sourceBounds = { x: lastX, y: lastY, width: draggedWin.size.width, height: draggedWin.size.height };
        move(id, bounds.x, bounds.y);
        resize(id, bounds.width, bounds.height);
        setWindowGridSlot(id, slot);
        move(otherId, sourceBounds.x, sourceBounds.y);
        resize(otherId, sourceBounds.width, sourceBounds.height);
        setWindowGridSlot(otherId, draggedWin.gridSlot);
      } else {
        move(id, bounds.x, bounds.y);
        resize(id, bounds.width, bounds.height);
        setWindowGridSlot(id, slot);
      }
    },
    [
      workspaceDropTarget,
      gridModeEnabled,
      layoutPreset,
      activeWorkspaceId,
      getSlotAtPoint,
      getOccupiedSlots,
      getFirstFreeSlot,
      getLayoutForWorkspace,
      moveWindowToWorkspace,
      windows,
      move,
      resize,
      setWindowGridSlot,
    ]
  );

  function renderWindowContent(win: { id: string; type: WindowType; title: string }) {
    if (win.type === "terminal") {
      return <Terminal username={username ?? "user"} />;
    }
    if (win.type === "theme") {
      return <ThemeApp />;
    }
    if (win.type === "explorer") {
      return <Explorer />;
    }
    if (win.type === "editor") {
      return <CodeEditor />;
    }
    if (win.type === "browser") {
      return <Browser />;
    }
    if (win.type === "sound") {
      return <SoundManager />;
    }
    if (win.type === "network") {
      return <NetworkManager />;
    }
    if (win.type === "email") {
      return <EmailApp />;
    }
    if (win.type === "wallet") {
      return <WalletApp />;
    }
    if (win.type === "pixelart") {
      return <PixelArtApp windowId={win.id} />;
    }
    if (win.type === "sysinfo") {
      return <SysinfoApp />;
    }
    if (win.type === "shortcuts") {
      return <ShortcutsApp />;
    }
    if (win.type === "sysmon") {
      return <SystemMonitorApp />;
    }
    if (win.type === "nullcloud") {
      return <NullCloudApp />;
    }
    return <PlaceholderContent title={win.title} />;
  }

  return (
    <ShortcutsProvider>
      <div className={styles.desktop}>
        <div className={styles.wallpaper} />
        <TopBar />
      {filePickerOpen && filePickerOptions && (
        <FilePicker
          open={true}
          mode={filePickerOptions.mode}
          initialPath={filePickerOptions.initialPath ?? getDefaultInitialPath()}
          onSelect={(path) => {
            filePickerOptions.onSelect(path);
            closeFilePicker();
          }}
          onCancel={() => {
            filePickerOptions.onCancel?.();
            closeFilePicker();
          }}
        />
      )}
      {appLauncherOpen && <AppLauncher />}
      {draggingWindowId && workspaceDropTarget && typeof window !== "undefined" && (
        <div className={styles.workspaceDropOverlay} aria-hidden>
          <div
            className={styles.workspaceDropGhostDot}
            style={{ left: dragCursorX, top: dragCursorY }}
          />
          <svg
            className={styles.workspaceDropLine}
            viewBox={`0 0 ${window.innerWidth} ${window.innerHeight}`}
            preserveAspectRatio="none"
          >
            <line
              x1={dragCursorX}
              y1={dragCursorY}
              x2={workspaceDropTarget.dotCenterX}
              y2={workspaceDropTarget.dotCenterY}
              stroke="var(--accent)"
              strokeWidth="2"
              strokeDasharray="6 4"
            />
          </svg>
        </div>
      )}
      <div className={styles.leftBottom} ref={leftBottomRef}>
        <LayoutPanel />
        <WorkspaceDots highlightedWorkspaceId={workspaceDropTarget?.workspaceId ?? null} />
      </div>
      <div className={styles.workspace}>
        {gridModeEnabled && snapPreview && (
          <div
            className={styles.gridSnapPreview}
            style={{
              left: snapPreview.x,
              top: snapPreview.y,
              width: snapPreview.width,
              height: snapPreview.height,
            }}
            aria-hidden
          />
        )}
        {visibleWindows.map((win) => (
          <Window
            key={win.id}
            id={win.id}
            title={win.title}
            position={win.position}
            size={win.size}
            onMove={move}
            onResize={resize}
            onClose={close}
            onMinimize={minimize}
            onMaximize={maximize}
            focused={focusedId === win.id}
            onFocus={() => setFocus(win.id)}
            minimized={win.minimized}
            maximized={win.maximized}
            zIndex={win.zIndex}
            icon={WINDOW_ICONS[win.type]}
            onDragStart={handleDragStart}
            onDragMove={handleDragMove}
            onDragEnd={handleDragEnd}
            dragGhost={draggingWindowId === win.id && workspaceDropTarget !== null}
          >
            {renderWindowContent(win)}
          </Window>
        ))}
      </div>
      <Dock username={username} />
      </div>
      <ShortcutsHandler />
    </ShortcutsProvider>
  );
}

export default function Desktop() {
  return (
    <WalletProvider>
      <NullCloudProvider>
        <PaymentFeedbackProvider>
          <WindowManagerProvider>
            <WorkspaceLayoutProvider>
              <FilePickerProvider>
                <AppLauncherProvider>
                  <DesktopContent />
                </AppLauncherProvider>
              </FilePickerProvider>
            </WorkspaceLayoutProvider>
          </WindowManagerProvider>
        </PaymentFeedbackProvider>
      </NullCloudProvider>
    </WalletProvider>
  );
}
