import React, {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
} from "react";

interface AuthContextValue {
  username: string | null;
  playerId: string | null;
  token: string | null;
  tokenExpiresAt: number | null;
  login: (
    username: string,
    playerId: string,
    token: string,
    expiresAt: number
  ) => void;
  logout: () => void;
  isTokenExpiringSoon: () => boolean;
}

const AuthContext = createContext<AuthContextValue | null>(null);

const TOKEN_STORAGE_KEY = "nulltrace_auth_token";
const TOKEN_EXPIRY_KEY = "nulltrace_token_expiry";
const USERNAME_STORAGE_KEY = "nulltrace_username";
const PLAYER_ID_STORAGE_KEY = "nulltrace_player_id";

function saveToken(
  token: string,
  expiresAt: number,
  username: string,
  playerId: string
) {
  localStorage.setItem(TOKEN_STORAGE_KEY, token);
  localStorage.setItem(TOKEN_EXPIRY_KEY, expiresAt.toString());
  localStorage.setItem(USERNAME_STORAGE_KEY, username);
  localStorage.setItem(PLAYER_ID_STORAGE_KEY, playerId);
}

function getToken(): {
  token: string;
  expiresAt: number;
  username: string;
  playerId: string;
} | null {
  const token = localStorage.getItem(TOKEN_STORAGE_KEY);
  const expiresAt = localStorage.getItem(TOKEN_EXPIRY_KEY);
  const username = localStorage.getItem(USERNAME_STORAGE_KEY);
  const playerId = localStorage.getItem(PLAYER_ID_STORAGE_KEY);
  if (!token || !expiresAt || !username || !playerId) return null;
  return {
    token,
    expiresAt: parseInt(expiresAt, 10),
    username,
    playerId,
  };
}

function clearToken() {
  localStorage.removeItem(TOKEN_STORAGE_KEY);
  localStorage.removeItem(TOKEN_EXPIRY_KEY);
  localStorage.removeItem(USERNAME_STORAGE_KEY);
  localStorage.removeItem(PLAYER_ID_STORAGE_KEY);
}

// Decode JWT to extract claims (client-side only, no verification)
function parseJwt(token: string): {
  sub: string;
  username: string;
  exp: number;
  iat: number;
} {
  const base64Url = token.split(".")[1];
  const base64 = base64Url.replace(/-/g, "+").replace(/_/g, "/");
  const jsonPayload = decodeURIComponent(
    atob(base64)
      .split("")
      .map((c) => "%" + ("00" + c.charCodeAt(0).toString(16)).slice(-2))
      .join("")
  );
  return JSON.parse(jsonPayload);
}

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [username, setUsername] = useState<string | null>(null);
  const [playerId, setPlayerId] = useState<string | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [tokenExpiresAt, setTokenExpiresAt] = useState<number | null>(null);

  // Load from localStorage on mount
  useEffect(() => {
    const stored = getToken();
    if (stored) {
      const now = Date.now() / 1000;
      if (stored.expiresAt > now) {
        // Token still valid
        setUsername(stored.username);
        setPlayerId(stored.playerId);
        setToken(stored.token);
        setTokenExpiresAt(stored.expiresAt);
      } else {
        // Token expired, clear
        clearToken();
      }
    }
  }, []);

  const login = useCallback(
    (name: string, id: string, tok: string, exp: number) => {
      setUsername(name);
      setPlayerId(id);
      setToken(tok);
      setTokenExpiresAt(exp);
      saveToken(tok, exp, name, id);
    },
    []
  );

  const logout = useCallback(() => {
    setUsername(null);
    setPlayerId(null);
    setToken(null);
    setTokenExpiresAt(null);
    clearToken();
  }, []);

  const isTokenExpiringSoon = useCallback(() => {
    if (!tokenExpiresAt) return false;
    const now = Date.now() / 1000;
    const twoHours = 2 * 60 * 60;
    return tokenExpiresAt - now < twoHours;
  }, [tokenExpiresAt]);

  return (
    <AuthContext.Provider
      value={{
        username,
        playerId,
        token,
        tokenExpiresAt,
        login,
        logout,
        isTokenExpiringSoon,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

// Export parseJwt for use in other components
export { parseJwt };

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used within AuthProvider");
  return ctx;
}
