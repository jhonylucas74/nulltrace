import { useState, useEffect, FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { useAuth } from "../contexts/AuthContext";
import styles from "./Login.module.css";

const MOCK_USERS = ["hacker", "root", "guest"];

function formatTime(d: Date): string {
  return d.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit", hour12: false });
}

function formatDate(d: Date): string {
  return d.toLocaleDateString("en-GB", { weekday: "long", day: "numeric", month: "long" });
}

export default function Login() {
  const [selectedUser, setSelectedUser] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  const [time, setTime] = useState(() => formatTime(new Date()));
  const [date, setDate] = useState(() => formatDate(new Date()));
  const { login } = useAuth();
  const navigate = useNavigate();

  useEffect(() => {
    const id = setInterval(() => setTime(formatTime(new Date())), 1000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    setDate(formatDate(new Date()));
  }, []);

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    const user = selectedUser ?? MOCK_USERS[0];
    login(user);
    navigate("/desktop", { replace: true });
  }

  function handleCreateUser() {
    // Mock: no-op or could show a message
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
              />
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
    </div>
  );
}
