import React, { createContext, useContext, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface LoginResponseMessage {
  success: boolean;
  player_id: string;
  token: string;
  error_message: string;
  preferred_theme?: string;
  shortcuts_overrides?: string;
}

export interface PingResponseMessage {
  server_time_ms: number;
}

export interface RefreshTokenResponse {
  success: boolean;
  token: string;
  error_message: string;
}

export interface GetPlayerProfileResponse {
  rank: number;
  points: number;
  faction_id: string;
  faction_name: string;
  preferred_theme: string;
  shortcuts_overrides: string;
  error_message: string;
}

export interface EmailMessage {
  id: string;
  from_address: string;
  to_address: string;
  subject: string;
  body: string;
  folder: string;
  read: boolean;
  sent_at_ms: number;
  /** CC recipients (optional; only set when email was sent with CC). */
  cc_address?: string;
}

// ── Wallet types ─────────────────────────────────────────────────────────────

export interface GrpcWalletBalance {
  currency: string;
  amount: number; // DB-cents; divide by 100 for display
}

export interface GrpcWalletTransaction {
  id: string;
  tx_type: string; // credit | debit | transfer_in | transfer_out | convert
  currency: string;
  amount: number; // DB-cents
  fee: number;
  description: string;
  counterpart_address: string;
  created_at_ms: number;
}

export interface GrpcWalletKey {
  currency: string;
  key_address: string;
}

export interface GrpcWalletCard {
  id: string;
  label: string;
  number_full: string;
  last4: string;
  expiry_month: number;
  expiry_year: number;
  cvv: string;
  holder_name: string;
  credit_limit: number; // DB-cents
  current_debt: number; // DB-cents
  is_virtual: boolean;
}

export interface GrpcCardTransaction {
  id: string;
  tx_type: string; // purchase | payment | refund
  amount: number; // DB-cents
  description: string;
  created_at_ms: number;
}

export interface GrpcCardStatement {
  id: string;
  card_id: string;
  period_start_ms: number;
  period_end_ms: number;
  total_amount: number; // DB-cents
  status: string; // open | closed | paid
  due_date_ms: number;
}

export interface GrpcContextValue {
  ping: () => Promise<PingResponseMessage>;
  login: (username: string, password: string) => Promise<LoginResponseMessage>;
  refreshToken: (currentToken: string) => Promise<RefreshTokenResponse>;
  getPlayerProfile: (token: string) => Promise<GetPlayerProfileResponse>;
  setPreferredTheme: (token: string, preferredTheme: string) => Promise<void>;
  setShortcuts: (token: string, shortcutsOverridesJson: string) => Promise<void>;
  getEmails: (
    emailAddress: string,
    mailToken: string,
    folder: string,
    page: number
  ) => Promise<{ emails: EmailMessage[]; hasMore: boolean }>;
  sendEmail: (
    fromAddress: string,
    mailToken: string,
    toAddress: string,
    subject: string,
    body: string,
    ccAddress?: string,
    bccAddress?: string
  ) => Promise<void>;
  markEmailRead: (emailAddress: string, mailToken: string, emailId: string, read: boolean) => Promise<void>;
  moveEmail: (emailAddress: string, mailToken: string, emailId: string, folder: string) => Promise<void>;
  deleteEmail: (emailAddress: string, mailToken: string, emailId: string) => Promise<void>;
  // ── Wallet ──────────────────────────────────────────────────────────────────
  getWalletBalances: (token: string) => Promise<{ balances: GrpcWalletBalance[]; error_message: string }>;
  getWalletTransactions: (token: string, filter: string) => Promise<{ transactions: GrpcWalletTransaction[]; error_message: string }>;
  getWalletKeys: (token: string) => Promise<{ keys: GrpcWalletKey[]; error_message: string }>;
  transferFunds: (token: string, targetAddress: string, currency: string, amount: number) => Promise<{ success: boolean; error_message: string }>;
  resolveTransferKey: (token: string, key: string) => Promise<{ is_valid: boolean; is_usd: boolean; account_holder_name: string; target_currency: string }>;
  convertFunds: (token: string, fromCurrency: string, toCurrency: string, amount: number) => Promise<{ success: boolean; converted_amount: number; error_message: string }>;
  getWalletCards: (token: string) => Promise<{ cards: GrpcWalletCard[]; error_message: string }>;
  createWalletCard: (token: string, label: string, creditLimit: number) => Promise<{ card: GrpcWalletCard | null; error_message: string }>;
  deleteWalletCard: (token: string, cardId: string) => Promise<{ success: boolean; error_message: string }>;
  getCardTransactions: (token: string, cardId: string, filter: string) => Promise<{ transactions: GrpcCardTransaction[]; error_message: string }>;
  getCardStatement: (token: string, cardId: string) => Promise<{ statement: GrpcCardStatement | null; error_message: string }>;
  payCardBill: (token: string, cardId: string) => Promise<{ success: boolean; amount_paid: number; error_message: string }>;
}

const GrpcContext = createContext<GrpcContextValue | null>(null);

export function GrpcProvider({ children }: { children: React.ReactNode }) {
  const value = useMemo<GrpcContextValue>(
    () => ({
      ping: () => invoke<PingResponseMessage>("grpc_ping"),
      login: (username: string, password: string) =>
        invoke<LoginResponseMessage>("grpc_login", { username, password }),
      refreshToken: (currentToken: string) =>
        invoke<RefreshTokenResponse>("grpc_refresh_token", { currentToken }),
      getPlayerProfile: (token: string) =>
        invoke<GetPlayerProfileResponse>("grpc_get_player_profile", { token }),
      setPreferredTheme: (token: string, preferredTheme: string) =>
        invoke<{ success: boolean; error_message: string }>("grpc_set_preferred_theme", {
          token,
          preferred_theme: preferredTheme,
        }).then((res) => {
          if (!res.success && res.error_message) {
            throw new Error(res.error_message);
          }
        }),
      setShortcuts: (token: string, shortcutsOverridesJson: string) =>
        invoke<{ success: boolean; error_message: string }>("grpc_set_shortcuts", {
          token,
          shortcuts_overrides_json: shortcutsOverridesJson,
        }).then((res) => {
          if (!res.success && res.error_message) {
            throw new Error(res.error_message);
          }
        }),
      getEmails: (emailAddress: string, mailToken: string, folder: string, page: number) =>
        invoke<{ emails: EmailMessage[]; hasMore: boolean }>("grpc_get_emails", {
          emailAddress,
          mailToken,
          folder,
          page,
        }),
      sendEmail: (
        fromAddress: string,
        mailToken: string,
        toAddress: string,
        subject: string,
        body: string,
        ccAddress?: string,
        bccAddress?: string
      ) =>
        invoke<{ success?: boolean; error_message?: string } | null>("grpc_send_email", {
          fromAddress,
          mailToken,
          toAddress,
          subject,
          body,
          ccAddress: ccAddress ?? null,
          bccAddress: bccAddress ?? null,
        }).then((res) => {
          if (res != null && res.success === false && res.error_message) {
            throw new Error(res.error_message);
          }
        }),
      markEmailRead: (emailAddress: string, mailToken: string, emailId: string, read: boolean) =>
        invoke<void>("grpc_mark_email_read", {
          emailAddress,
          mailToken,
          emailId,
          read,
        }),
      moveEmail: (emailAddress: string, mailToken: string, emailId: string, folder: string) =>
        invoke<void>("grpc_move_email", {
          emailAddress,
          mailToken,
          emailId,
          folder,
        }),
      deleteEmail: (emailAddress: string, mailToken: string, emailId: string) =>
        invoke<void>("grpc_delete_email", {
          emailAddress,
          mailToken,
          emailId,
        }),
      // ── Wallet ────────────────────────────────────────────────────────────
      getWalletBalances: (token: string) =>
        invoke<{ balances: GrpcWalletBalance[]; error_message: string }>("grpc_get_wallet_balances", { token }),
      getWalletTransactions: (token: string, filter: string) =>
        invoke<{ transactions: GrpcWalletTransaction[]; error_message: string }>("grpc_get_wallet_transactions", { token, filter }),
      getWalletKeys: (token: string) =>
        invoke<{ keys: GrpcWalletKey[]; error_message: string }>("grpc_get_wallet_keys", { token }),
      transferFunds: (token: string, targetAddress: string, currency: string, amount: number) =>
        invoke<{ success: boolean; error_message: string }>("grpc_transfer_funds", { token, targetAddress, currency, amount }),
      resolveTransferKey: (token: string, key: string) =>
        invoke<{ is_valid: boolean; is_usd: boolean; account_holder_name: string; target_currency: string }>("grpc_resolve_transfer_key", { token, key }),
      convertFunds: (token: string, fromCurrency: string, toCurrency: string, amount: number) =>
        invoke<{ success: boolean; converted_amount: number; error_message: string }>("grpc_convert_funds", { token, fromCurrency, toCurrency, amount }),
      getWalletCards: (token: string) =>
        invoke<{ cards: GrpcWalletCard[]; error_message: string }>("grpc_get_wallet_cards", { token }),
      createWalletCard: (token: string, label: string, creditLimit: number) =>
        invoke<{ card: GrpcWalletCard | null; error_message: string }>("grpc_create_wallet_card", { token, label, creditLimit }),
      deleteWalletCard: (token: string, cardId: string) =>
        invoke<{ success: boolean; error_message: string }>("grpc_delete_wallet_card", { token, cardId }),
      getCardTransactions: (token: string, cardId: string, filter: string) =>
        invoke<{ transactions: GrpcCardTransaction[]; error_message: string }>("grpc_get_card_transactions", { token, cardId, filter }),
      getCardStatement: (token: string, cardId: string) =>
        invoke<{ statement: GrpcCardStatement | null; error_message: string }>("grpc_get_card_statement", { token, cardId }),
      payCardBill: (token: string, cardId: string) =>
        invoke<{ success: boolean; amount_paid: number; error_message: string }>("grpc_pay_card_bill", { token, cardId }),
    }),
    []
  );

  return <GrpcContext.Provider value={value}>{children}</GrpcContext.Provider>;
}

export function useGrpc(): GrpcContextValue {
  const ctx = useContext(GrpcContext);
  if (!ctx) throw new Error("useGrpc must be used within GrpcProvider");
  return ctx;
}
