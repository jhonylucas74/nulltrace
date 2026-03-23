import { useState, FormEvent, useEffect } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { Shield } from "lucide-react";
import { useAuth } from "../contexts/AuthContext";
import styles from "./Login.module.css";

export default function Login() {
  const [email, setEmail] = useState("admin");
  const [password, setPassword] = useState("admin");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const { login } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();

  useEffect(() => {
    const state = location.state as { sessionExpired?: boolean } | null;
    if (state?.sessionExpired) {
      setError("Session expired. Please sign in again.");
    }
  }, [location.state]);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    console.log("[admin_login] handleSubmit called");
    setError(null);
    setSubmitting(true);
    console.log("[admin_login] before invoke", { username: email, passwordLen: password?.length });
    try {
      const res = await invoke<{ success: boolean; token: string; error_message: string }>(
        "admin_login",
        { email, password }
      );
      console.log("[admin_login] invoke success", {
        success: res.success,
        hasToken: !!res.token,
        error_message: res.error_message,
      });
      if (res.success && res.token) {
        login(res.token);
        navigate("/", { replace: true });
      } else {
        setError(res.error_message || "Invalid username or password");
      }
    } catch (err) {
      console.error("[admin_login] invoke error", err);
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className={styles.container}>
      <div className={styles.card}>
        <div className={styles.header}>
          <Shield size={40} className={styles.icon} />
          <h1 className={styles.title}>Nulltrace Admin</h1>
          <p className={styles.subtitle}>Cluster management dashboard</p>
        </div>
        <form onSubmit={handleSubmit} className={styles.form}>
          <div className={styles.field}>
            <label htmlFor="username">Username</label>
            <input
              id="username"
              type="text"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="admin"
              autoComplete="username"
              disabled={submitting}
            />
          </div>
          <div className={styles.field}>
            <label htmlFor="password">Password</label>
            <input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="••••••••"
              autoComplete="current-password"
              disabled={submitting}
            />
          </div>
          {error && <p className={styles.error}>{error}</p>}
          <button type="submit" className={styles.submit} disabled={submitting}>
            {submitting ? "Signing in…" : "Sign in"}
          </button>
        </form>
      </div>
    </div>
  );
}
