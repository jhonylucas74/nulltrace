import React, { createContext, useContext, useState, useCallback } from "react";

interface AuthContextValue {
  username: string | null;
  playerId: string | null;
  login: (username: string, playerId?: string) => void;
  logout: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [username, setUsername] = useState<string | null>(null);
  const [playerId, setPlayerId] = useState<string | null>(null);

  const login = useCallback((name: string, id?: string) => {
    setUsername(name.trim() || null);
    setPlayerId(id ?? null);
  }, []);

  const logout = useCallback(() => {
    setUsername(null);
    setPlayerId(null);
  }, []);

  return (
    <AuthContext.Provider value={{ username, playerId, login, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used within AuthProvider");
  return ctx;
}
