import { useEffect, useRef } from "react";
import styles from "./Modal.module.css";

export interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  primaryButton?: { label: string; onClick: () => void };
  secondaryButton?: { label: string; onClick: () => void };
}

export default function Modal({
  open,
  onClose,
  title,
  children,
  primaryButton,
  secondaryButton,
}: ModalProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function handleEscape(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [open, onClose]);

  if (!open) return null;

  function handleOverlayClick(e: React.MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  return (
    <div className={styles.overlay} onClick={handleOverlayClick} role="dialog" aria-modal="true" aria-labelledby="modal-title">
      <div ref={panelRef} className={styles.panel} onClick={(e) => e.stopPropagation()}>
        <div className={styles.titleBar}>
          <h2 id="modal-title" className={styles.title}>
            {title}
          </h2>
        </div>
        <div className={styles.content}>{children}</div>
        <div className={styles.footer}>
          {secondaryButton && (
            <button type="button" className={styles.secondaryBtn} onClick={secondaryButton.onClick}>
              {secondaryButton.label}
            </button>
          )}
          {primaryButton && (
            <button type="button" className={styles.primaryBtn} onClick={primaryButton.onClick}>
              {primaryButton.label}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
