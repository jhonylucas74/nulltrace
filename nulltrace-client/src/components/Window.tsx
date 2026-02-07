import { useRef, useState, useCallback } from "react";
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
  onClose: (id: string) => void;
  onMinimize: (id: string) => void;
  onMaximize: (id: string) => void;
  focused?: boolean;
  onFocus?: () => void;
  minimized?: boolean;
  maximized?: boolean;
  zIndex: number;
  children: React.ReactNode;
}

function isTitleBarButton(el: HTMLElement | null): boolean {
  if (!el) return false;
  const target = el as HTMLElement;
  return !!(
    target.closest(`.${styles.minBtn}`) ||
    target.closest(`.${styles.maxBtn}`) ||
    target.closest(`.${styles.closeBtn}`)
  );
}

export default function Window({
  id,
  title,
  icon,
  position,
  size,
  onMove,
  onClose,
  onMinimize,
  onMaximize,
  focused = false,
  onFocus,
  minimized = false,
  maximized = false,
  zIndex,
  children,
}: WindowProps) {
  const [isDragging, setIsDragging] = useState(false);
  const dragStart = useRef({ x: 0, y: 0, posX: 0, posY: 0 });

  const handleTitlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (maximized) return;
      if (isTitleBarButton((e.target as HTMLElement))) return;
      e.preventDefault();
      setIsDragging(true);
      dragStart.current = {
        x: e.clientX,
        y: e.clientY,
        posX: position.x,
        posY: position.y,
      };
      onFocus?.();
      (e.target as HTMLElement).setPointerCapture?.(e.pointerId);
    },
    [position, onFocus, maximized]
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!isDragging) return;
      const dx = e.clientX - dragStart.current.x;
      const dy = e.clientY - dragStart.current.y;
      onMove(id, dragStart.current.posX + dx, dragStart.current.posY + dy);
    },
    [id, isDragging, onMove]
  );

  const handlePointerUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  if (minimized) {
    return null;
  }

  const style: React.CSSProperties = maximized
    ? { zIndex }
    : {
        left: position.x,
        top: position.y,
        width: size.width,
        height: size.height,
        zIndex,
      };

  return (
    <div
      className={`${styles.window} ${focused ? styles.focused : ""} ${maximized ? styles.maximized : ""}`}
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
    </div>
  );
}
