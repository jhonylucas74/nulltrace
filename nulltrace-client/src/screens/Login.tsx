import { useState, useEffect, useRef, FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { Power } from "lucide-react";
import { useAuth } from "../contexts/AuthContext";
import { useGrpc } from "../contexts/GrpcContext";
import { useTheme, VALID_THEMES } from "../contexts/ThemeContext";
import type { ThemeId } from "../contexts/ThemeContext";
import styles from "./Login.module.css";

/** Available users on the OS simulator (selectable cards). Haru only for now. */
const MOCK_USERS = ["Haru"];

function formatTime(d: Date): string {
  return d.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit", hour12: false });
}

function formatDate(d: Date): string {
  return d.toLocaleDateString("en-GB", { weekday: "long", day: "numeric", month: "long" });
}

export default function Login() {
  const [selectedUser, setSelectedUser] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [serverReachable, setServerReachable] = useState<boolean | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [time, setTime] = useState(() => formatTime(new Date()));
  const [date, setDate] = useState(() => formatDate(new Date()));
  const [powerMenuOpen, setPowerMenuOpen] = useState(false);
  const powerMenuRef = useRef<HTMLDivElement>(null);
  const { login } = useAuth();
  const { login: grpcLogin, ping } = useGrpc();
  const { setTheme } = useTheme();
  const navigate = useNavigate();

  useEffect(() => {
    let cancelled = false;
    ping()
      .then(() => {
        if (!cancelled) setServerReachable(true);
      })
      .catch(() => {
        if (!cancelled) setServerReachable(false);
      });
    return () => {
      cancelled = true;
    };
  }, [ping]);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (powerMenuRef.current && !powerMenuRef.current.contains(e.target as Node)) {
        setPowerMenuOpen(false);
      }
    }
    if (powerMenuOpen) {
      document.addEventListener("mousedown", handleClickOutside);
      return () => document.removeEventListener("mousedown", handleClickOutside);
    }
  }, [powerMenuOpen]);

  async function handleQuitGame() {
    setPowerMenuOpen(false);
    const tauri = (window as unknown as { __TAURI__?: unknown }).__TAURI__;
    if (tauri) {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().destroy();
    } else {
      window.close();
    }
  }

  useEffect(() => {
    const id = setInterval(() => setTime(formatTime(new Date())), 1000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    setDate(formatDate(new Date()));
  }, []);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    const user = selectedUser ?? MOCK_USERS[0];
    setSubmitting(true);
    try {
      const res = await grpcLogin(user, password);
      if (res.success) {
        // Parse token to get expiry
        const { parseJwt } = await import("../contexts/AuthContext");
        const claims = parseJwt(res.token);
        login(user, res.player_id, res.token, claims.exp);
        // Only apply server theme when it's a real saved preference (not default), so we don't
        // overwrite the user's localStorage choice with "githubdark" after login.
        const serverTheme = res.preferred_theme?.trim();
        if (
          serverTheme &&
          VALID_THEMES.includes(serverTheme as ThemeId) &&
          serverTheme !== "githubdark"
        ) {
          setTheme(serverTheme as ThemeId);
        }
        navigate("/desktop", { replace: true });
      } else {
        setError(res.error_message || "Invalid credentials");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Cannot reach server");
    } finally {
      setSubmitting(false);
    }
  }

  function handleCreateUser() {
    // Mock: no-op for now
  }

  return (
    <div className={styles.wrapper}>
      <div className={styles.bg} />

      <div className={styles.clockBlock}>
        <div className={styles.clock}>{time}</div>
        <div className={styles.date}>{date}</div>
      </div>

      <div className={styles.center}>
        <img src="/logo.png" alt="" className={styles.logo} />
        <h1 className={styles.logoWordmark}>
          <span className={styles.logoWordmarkNull}>null</span>
          <span className={styles.logoWordmarkTrace}>trace</span>
        </h1>
        {serverReachable === false && (
          <p className={styles.errorText}>Cannot reach server. Check that the backend is running.</p>
        )}
        {!selectedUser ? (
          <div className={styles.userList}>
            {MOCK_USERS.map((user) => (
              <button
                key={user}
                type="button"
                className={`${styles.userCard} ${selectedUser === user ? styles.userCardSelected : ""}`}
                onClick={() => setSelectedUser(user)}
              >
                <span className={styles.avatar}>{user.charAt(0)}</span>
                <span className={styles.userName}>{user}</span>
              </button>
            ))}
          </div>
        ) : (
          <>
            <form onSubmit={handleSubmit} className={styles.passwordForm}>
              <input
                type="password"
                placeholder="Password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className={styles.input}
                autoComplete="current-password"
                autoFocus
                disabled={submitting}
              />
              {error && <p className={styles.errorText}>{error}</p>}
              <p className={styles.sessionLabel}>Session: Nulltrace · Press Enter to sign in</p>
            </form>
            <div className={styles.userCircles}>
              {MOCK_USERS.map((user) => (
                <button
                  key={user}
                  type="button"
                  className={`${styles.userCircle} ${selectedUser === user ? styles.userCircleSelected : ""}`}
                  onClick={() => setSelectedUser(user)}
                  title={user}
                >
                  <span className={styles.userCircleLetter}>{user.charAt(0)}</span>
                </button>
              ))}
            </div>
          </>
        )}
      </div>

      <button type="button" className={styles.createUser} onClick={handleCreateUser}>
        Create new user
      </button>

      <div className={styles.powerWrap} ref={powerMenuRef}>
        <button
          type="button"
          className={styles.powerBtn}
          onClick={() => setPowerMenuOpen((o) => !o)}
          title="Power options"
          aria-label="Power options"
          aria-expanded={powerMenuOpen}
          aria-haspopup="true"
        >
          <Power size={22} />
        </button>
        {powerMenuOpen && (
          <div className={styles.powerDropdown}>
            <button
              type="button"
              className={styles.powerDropdownItem}
              onClick={handleQuitGame}
            >
              Quit game
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
