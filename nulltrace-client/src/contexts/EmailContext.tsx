import React, { createContext, useContext, useState, useEffect, useRef, type Dispatch, type SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAuth } from "./AuthContext";

/** Matches Tauri bridge emit: { type, count } or { type, email }. Backward compat: unread_count / new_email. */
interface MailboxEventPayload {
  type?: string;
  count?: number;
  email?: {
    id: string;
    from_address: string;
    to_address: string;
    subject: string;
    body: string;
    folder: string;
    read: boolean;
    sent_at_ms: number;
  };
  unread_count?: number;
  new_email?: MailboxEventPayload["email"];
}

interface ReadFileResponse {
  success: boolean;
  error_message: string;
  content: string;
}

interface EmailContextValue {
  emailAddress: string | null;
  mailToken: string | null;
  unreadCount: number;
  setUnreadCount: Dispatch<SetStateAction<number>>;
  /** Increments when a new_email event is received; use in EmailApp to refetch inbox. */
  inboxInvalidated: number;
  isReady: boolean;
}

const EmailContext = createContext<EmailContextValue | null>(null);

export function EmailProvider({ children }: { children: React.ReactNode }) {
  const { token, username } = useAuth();
  const [emailAddress, setEmailAddress] = useState<string | null>(null);
  const [mailToken, setMailToken] = useState<string | null>(null);
  const [unreadCount, setUnreadCount] = useState(0);
  const [inboxInvalidated, setInboxInvalidated] = useState(0);
  const [isReady, setIsReady] = useState(false);
  const connIdRef = useRef<string | null>(null);

  useEffect(() => {
    if (!token || !username) {
      console.log("[EmailContext] skip init: missing token or username", { hasToken: !!token, hasUsername: !!username });
      return;
    }

    const currentUsername = username;
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    async function init() {
      const log = (msg: string, data?: unknown) => {
        console.log("[EmailContext]", msg, data !== undefined ? data : "");
      };
      try {
        log("init started", { username: currentUsername });
        // Layout: /etc/mail/<address>/token (address = username@mail.null for player VMs)
        const address = `${currentUsername.toLowerCase()}@mail.null`;
        const tokenPath = `/etc/mail/${address}/token`;
        const tokenResult = await invoke<ReadFileResponse>("grpc_read_file", {
          path: tokenPath,
          token,
        });
        if (cancelled) return;
        log("read token", {
          path: tokenPath,
          success: tokenResult.success,
          error: tokenResult.error_message || undefined,
          hasContent: !!(tokenResult.content?.trim()),
        });
        if (!tokenResult.success || !tokenResult.content.trim()) {
          log("init aborted: token file missing or failed");
          return;
        }
        const mailTok = tokenResult.content.trim();

        if (cancelled) return;

        setMailToken(mailTok);
        setEmailAddress(address);
        log("mailbox_connect starting", { address });
        const connId = await invoke<string>("mailbox_connect", {
          emailAddress: address,
          mailToken: mailTok,
        });

        if (cancelled) {
          invoke("mailbox_disconnect", { connId }).catch(() => {});
          return;
        }
        connIdRef.current = connId;
        log("mailbox_connect ok", { connId });

        unlisten = await listen<MailboxEventPayload>("mailbox_event", (event) => {
          const payload = event.payload;
          if (payload.type === "unread_count" && typeof payload.count === "number") {
            setUnreadCount(payload.count);
          } else if (payload.type === "new_email" && payload.email) {
            setUnreadCount((prev) => prev + 1);
            setInboxInvalidated((prev) => prev + 1);
          } else if (typeof payload.unread_count === "number") {
            setUnreadCount(payload.unread_count);
          } else if (payload.new_email) {
            setUnreadCount((prev) => prev + 1);
            setInboxInvalidated((prev) => prev + 1);
          }
        });
        log("mailbox_event listener attached");
        setIsReady(true);
        log("init completed", { address });
      } catch (e) {
        console.error("[EmailContext] init failed:", e);
      }
    }

    init();

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
      if (connIdRef.current) {
        invoke("mailbox_disconnect", { connId: connIdRef.current }).catch(() => {});
        connIdRef.current = null;
      }
      setEmailAddress(null);
      setMailToken(null);
      setUnreadCount(0);
      setIsReady(false);
    };
  }, [token, username]);

  return (
    <EmailContext.Provider value={{ emailAddress, mailToken, unreadCount, setUnreadCount, inboxInvalidated, isReady }}>
      {children}
    </EmailContext.Provider>
  );
}

export function useEmail(): EmailContextValue {
  const ctx = useContext(EmailContext);
  if (!ctx) throw new Error("useEmail must be used within EmailProvider");
  return ctx;
}
