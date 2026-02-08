import { useRef, useState, useCallback, useEffect } from "react";
import { useWorkspaceLayout } from "../contexts/WorkspaceLayoutContext";
import styles from "./Window.module.css";

export interface WindowPosition {
  x: number;
  y: number;
}

export interface WindowSize {
  width: number;
  height: number;
}

interface WindowProps {
  id: string;
  title: string;
  icon?: React.ReactNode;
  position: WindowPosition;
  size: WindowSize;
  onMove: (id: string, x: number, y: number) => void;
  onResize: (id: string, width: number, height: number) => void;
  onClose: (id: string) => void;
  onMinimize: (id: string) => void;
  onMaximize: (id: string) => void;
  focused?: boolean;
  onFocus?: () => void;
  minimized?: boolean;
  maximized?: boolean;
  zIndex: number;
  onDragStart?: (id: string) => void;
  onDragMove?: (id: string, clientX: number, clientY: number) => void;
  onDragEnd?: (id: string, lastX: number, lastY: number, centerClientX: number, centerClientY: number) => void;
  style?: React.CSSProperties;
  /** When true, window is hidden (used when dragging to workspace dots). */
  dragGhost?: boolean;
  children: React.ReactNode;
}

function isTitleBarButton(el: HTMLElement | null): boolean {
  if (!el) return false;
  const target = el as HTMLElement;
  return !!(
    target.closest(`.${styles.minBtn}`) ||
    target.closest(`.${styles.maxBtn}`) ||
    target.closest(`.${styles.closeBtn}`) ||
    target.closest(`.${styles.workspaceMenuWrap}`)
  );
}

export default function Window({
  id,
  title,
  icon,
  position,
  size,
  onMove,
  onResize,
  onClose,
  onMinimize,
  onMaximize,
  focused = false,
  onFocus,
  minimized = false,
  maximized = false,
  zIndex,
  onDragStart,
  onDragMove,
  onDragEnd,
  style: styleProp,
  dragGhost = false,
  children,
}: WindowProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [isResizing, setIsResizing] = useState(false);
  const [workspaceMenuOpen, setWorkspaceMenuOpen] = useState(false);
  const dragStart = useRef({ x: 0, y: 0, posX: 0, posY: 0 });
  const resizeStart = useRef({ x: 0, y: 0, width: 0, height: 0 });
  const workspaceMenuRef = useRef<HTMLDivElement>(null);
  const { workspaces, moveWindowToWorkspace } = useWorkspaceLayout();

  useEffect(() => {
    if (!workspaceMenuOpen) return;
    function handleClickOutside(e: MouseEvent) {
      if (workspaceMenuRef.current && !workspaceMenuRef.current.contains(e.target as Node)) {
        setWorkspaceMenuOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [workspaceMenuOpen]);

  const MIN_W = 320;
  const MIN_H = 200;

  const handleTitlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (maximized) return;
      if (isTitleBarButton((e.target as HTMLElement))) return;
      e.preventDefault();
      setIsDragging(true);
      onDragStart?.(id);
      dragStart.current = {
        x: e.clientX,
        y: e.clientY,
        posX: position.x,
        posY: position.y,
      };
      onFocus?.();
      (e.target as HTMLElement).setPointerCapture?.(e.pointerId);
    },
    [id, position, onFocus, maximized, onDragStart]
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!isDragging) return;
      const dx = e.clientX - dragStart.current.x;
      const dy = e.clientY - dragStart.current.y;
      onMove(id, dragStart.current.posX + dx, dragStart.current.posY + dy);
      onDragMove?.(id, e.clientX, e.clientY);
    },
    [id, isDragging, onMove, onDragMove]
  );

  const handlePointerUp = useCallback(
    (_e: React.PointerEvent) => {
      if (isDragging && onDragEnd) {
        const centerClientX = position.x + size.width / 2;
        const centerClientY = position.y + size.height / 2;
        onDragEnd(id, position.x, position.y, centerClientX, centerClientY);
      }
      setIsDragging(false);
      setIsResizing(false);
    },
    [id, isDragging, position, size, onDragEnd]
  );

  const handleResizePointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (maximized) return;
      e.preventDefault();
      e.stopPropagation();
      setIsResizing(true);
      resizeStart.current = {
        x: e.clientX,
        y: e.clientY,
        width: size.width,
        height: size.height,
      };
      onFocus?.();
      (e.target as HTMLElement).setPointerCapture?.(e.pointerId);
    },
    [maximized, size, onFocus]
  );

  const handleResizePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!isResizing) return;
      const dx = e.clientX - resizeStart.current.x;
      const dy = e.clientY - resizeStart.current.y;
      const w = Math.max(MIN_W, resizeStart.current.width + dx);
      const h = Math.max(MIN_H, resizeStart.current.height + dy);
      onResize(id, w, h);
    },
    [id, isResizing, onResize]
  );

  if (minimized) {
    return null;
  }

  const style: React.CSSProperties = {
    ...(maximized
      ? { zIndex }
      : {
          left: position.x,
          top: position.y,
          width: size.width,
          height: size.height,
          zIndex,
        }),
    ...(styleProp ?? {}),
  };

  return (
    <div
      className={`${styles.window} ${focused ? styles.focused : ""} ${maximized ? styles.maximized : ""} ${dragGhost ? styles.dragGhost : ""}`}
      style={style}
      onPointerDown={onFocus}
    >
      <div
        className={styles.titleBar}
        onPointerDown={handleTitlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerLeave={handlePointerUp}
      >
        {icon && <span className={styles.icon}>{icon}</span>}
        <span className={styles.title}>{title}</span>
        <div className={styles.controls}>
          <div className={styles.workspaceMenuWrap} ref={workspaceMenuRef}>
            <button
              type="button"
              className={styles.workspaceMenuBtn}
              onClick={() => setWorkspaceMenuOpen((o) => !o)}
              aria-label="Move to workspace"
              aria-expanded={workspaceMenuOpen}
              aria-haspopup="true"
              title="Move to workspace"
            >
              ⋯
            </button>
            {workspaceMenuOpen && (
              <div className={styles.workspaceDropdown} role="menu">
                <div className={styles.workspaceDropdownLabel}>Move to workspace</div>
                {workspaces.map((ws) => (
                  <button
                    key={ws.id}
                    type="button"
                    role="menuitem"
                    className={styles.workspaceDropdownItem}
                    onClick={() => {
                      moveWindowToWorkspace(id, ws.id);
                      setWorkspaceMenuOpen(false);
                    }}
                  >
                    {ws.label}
                  </button>
                ))}
              </div>
            )}
          </div>
          <button
            type="button"
            className={styles.minBtn}
            onClick={() => onMinimize(id)}
            aria-label="Minimize"
          >
            —
          </button>
          <button
            type="button"
            className={styles.maxBtn}
            onClick={() => onMaximize(id)}
            aria-label={maximized ? "Restore" : "Maximize"}
          >
            {maximized ? "❐" : "□"}
          </button>
          <button
            type="button"
            className={styles.closeBtn}
            onClick={() => onClose(id)}
            aria-label="Close"
          >
            ×
          </button>
        </div>
      </div>
      <div className={styles.content}>{children}</div>
      {!maximized && (
        <div
          className={styles.resizeHandle}
          onPointerDown={handleResizePointerDown}
          onPointerMove={handleResizePointerMove}
          onPointerUp={handlePointerUp}
          onPointerLeave={handlePointerUp}
          aria-label="Resize"
        />
      )}
    </div>
  );
}
