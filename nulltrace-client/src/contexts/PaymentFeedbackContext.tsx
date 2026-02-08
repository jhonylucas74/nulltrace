/**
 * Payment feedback: fly-to-wallet effect. When a payment is made, a green ball
 * flies from the click position to the Wallet icon in the dock; on arrival the
 * icon gets a brief effect. Reusable from any app that calls wallet.pay().
 */

import React, {
  createContext,
  useContext,
  useCallback,
  useRef,
  useState,
  useEffect,
  useLayoutEffect,
} from "react";
import { createPortal } from "react-dom";

const FLY_DURATION_MS = 320;
const IMPACT_DURATION_MS = 320;
const BALL_SIZE = 12;

export interface PaymentFeedbackContextValue {
  triggerFlyToWallet: (originClientX: number, originClientY: number) => void;
  registerWalletIconElement: (el: HTMLElement | null) => void;
  walletIconImpact: boolean;
}

const PaymentFeedbackContext = createContext<PaymentFeedbackContextValue | null>(null);

function FlyingBallOverlay({
  isFlying,
  startX,
  startY,
  endX,
  endY,
  onTransitionEnd,
}: {
  isFlying: boolean;
  startX: number;
  startY: number;
  endX: number;
  endY: number;
  onTransitionEnd: () => void;
}) {
  const [position, setPosition] = useState({ x: startX, y: startY, opacity: 1 });
  const hasTriggeredEnd = useRef(false);

  useLayoutEffect(() => {
    if (!isFlying) return;
    setPosition({ x: startX, y: startY, opacity: 1 });
    hasTriggeredEnd.current = false;
  }, [isFlying, startX, startY]);

  useEffect(() => {
    if (!isFlying) return;
    const id = requestAnimationFrame(() => {
      setPosition({ x: endX, y: endY, opacity: 0 });
    });
    return () => cancelAnimationFrame(id);
  }, [isFlying, endX, endY]);

  const handleTransitionEnd = useCallback(
    (e: React.TransitionEvent) => {
      if (e.propertyName !== "left" && e.propertyName !== "top") return;
      if (hasTriggeredEnd.current) return;
      hasTriggeredEnd.current = true;
      onTransitionEnd();
    },
    [onTransitionEnd]
  );

  if (!isFlying) return null;

  return createPortal(
    <div
      role="presentation"
      aria-hidden
      style={{
        position: "fixed",
        left: position.x,
        top: position.y,
        width: BALL_SIZE,
        height: BALL_SIZE,
        marginLeft: -BALL_SIZE / 2,
        marginTop: -BALL_SIZE / 2,
        borderRadius: "50%",
        background: "var(--term-green, #50fa7b)",
        boxShadow: "0 0 12px var(--term-green, #50fa7b)",
        opacity: position.opacity,
        pointerEvents: "none",
        zIndex: 9999,
        transition: `left ${FLY_DURATION_MS}ms ease-in, top ${FLY_DURATION_MS}ms ease-in, opacity ${FLY_DURATION_MS}ms ease-in`,
      }}
      onTransitionEnd={handleTransitionEnd}
    />,
    document.body
  );
}

export function PaymentFeedbackProvider({ children }: { children: React.ReactNode }) {
  const walletIconRef = useRef<HTMLElement | null>(null);
  const [isFlying, setIsFlying] = useState(false);
  const [walletIconImpact, setWalletIconImpact] = useState(false);
  const [flyCoords, setFlyCoords] = useState({ startX: 0, startY: 0, endX: 0, endY: 0 });

  const registerWalletIconElement = useCallback((el: HTMLElement | null) => {
    walletIconRef.current = el;
  }, []);

  const triggerFlyToWallet = useCallback((originClientX: number, originClientY: number) => {
    const el = walletIconRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const endX = rect.left + rect.width / 2;
    const endY = rect.top + rect.height / 2;
    setFlyCoords({
      startX: originClientX,
      startY: originClientY,
      endX,
      endY,
    });
    setIsFlying(true);
  }, []);

  const handleFlyTransitionEnd = useCallback(() => {
    setWalletIconImpact(true);
    window.setTimeout(() => {
      setIsFlying(false);
      setWalletIconImpact(false);
    }, IMPACT_DURATION_MS);
  }, []);

  const value: PaymentFeedbackContextValue = {
    triggerFlyToWallet,
    registerWalletIconElement,
    walletIconImpact,
  };

  return (
    <PaymentFeedbackContext.Provider value={value}>
      {children}
      <FlyingBallOverlay
        isFlying={isFlying}
        startX={flyCoords.startX}
        startY={flyCoords.startY}
        endX={flyCoords.endX}
        endY={flyCoords.endY}
        onTransitionEnd={handleFlyTransitionEnd}
      />
    </PaymentFeedbackContext.Provider>
  );
}

export function usePaymentFeedback(): PaymentFeedbackContextValue {
  const ctx = useContext(PaymentFeedbackContext);
  if (!ctx) throw new Error("usePaymentFeedback must be used within PaymentFeedbackProvider");
  return ctx;
}

/** Optional hook for components that may not be inside the provider (e.g. Dock). */
export function usePaymentFeedbackOptional(): PaymentFeedbackContextValue | null {
  return useContext(PaymentFeedbackContext);
}
