import React, { createContext, useContext, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface LoginResponseMessage {
  success: boolean;
  player_id: string;
  error_message: string;
}

export interface PingResponseMessage {
  server_time_ms: number;
}

export interface GrpcContextValue {
  ping: () => Promise<PingResponseMessage>;
  login: (username: string, password: string) => Promise<LoginResponseMessage>;
}

const GrpcContext = createContext<GrpcContextValue | null>(null);

export function GrpcProvider({ children }: { children: React.ReactNode }) {
  const value = useMemo<GrpcContextValue>(() => ({
    ping: () => invoke<PingResponseMessage>("grpc_ping"),
    login: (username: string, password: string) =>
      invoke<LoginResponseMessage>("grpc_login", { username, password }),
  }), []);

  return <GrpcContext.Provider value={value}>{children}</GrpcContext.Provider>;
}

export function useGrpc(): GrpcContextValue {
  const ctx = useContext(GrpcContext);
  if (!ctx) throw new Error("useGrpc must be used within GrpcProvider");
  return ctx;
}
