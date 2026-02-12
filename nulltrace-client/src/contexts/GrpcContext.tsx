import React, { createContext, useContext, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface LoginResponseMessage {
  success: boolean;
  player_id: string;
  token: string;
  error_message: string;
}

export interface PingResponseMessage {
  server_time_ms: number;
}

export interface RefreshTokenResponse {
  success: boolean;
  token: string;
  error_message: string;
}

export interface GrpcContextValue {
  ping: () => Promise<PingResponseMessage>;
  login: (username: string, password: string) => Promise<LoginResponseMessage>;
  refreshToken: (currentToken: string) => Promise<RefreshTokenResponse>;
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
