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

export interface GrpcContextValue {
  ping: () => Promise<PingResponseMessage>;
  login: (username: string, password: string) => Promise<LoginResponseMessage>;
  refreshToken: (currentToken: string) => Promise<RefreshTokenResponse>;
  getPlayerProfile: (token: string) => Promise<GetPlayerProfileResponse>;
  setPreferredTheme: (token: string, preferredTheme: string) => Promise<void>;
  setShortcuts: (token: string, shortcutsOverridesJson: string) => Promise<void>;
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
