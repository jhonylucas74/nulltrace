import { useTheme } from "../contexts/ThemeContext";
import type { ThemeId } from "../contexts/ThemeContext";
import { useAuth } from "../contexts/AuthContext";
import { NULLTRACE_ASCII_ART, NULLTRACE_ASCII_ART_INVERTED } from "../lib/nulltraceAsciiArt";
import styles from "./SysinfoApp.module.css";

const THEME_DISPLAY_NAMES: Record<ThemeId, string> = {
  latte: "Latte",
  frappe: "Frappé",
  macchiato: "Macchiato",
  mocha: "Mocha",
  onedark: "One Dark",
  dracula: "Dracula",
  githubdark: "Nulltrace",
  monokai: "Monokai",
  solardark: "Solarized Dark",
};

/** Mock leaderboard/faction until API exists. */
const MOCK_LEADERBOARD_RANK = 42;
const MOCK_FACTION = "Neon Syndicate";

/** Fake system info (no real PC data). */
const MOCK_CPU = "4 cores";
const MOCK_MEMORY = "4.2 GiB / 15.8 GiB";
const MOCK_DISK = "12 GiB used, 88 GiB free";
const MOCK_VERSION = "0.1.0";

export default function SysinfoApp() {
  const { theme } = useTheme();
  const { username } = useAuth();

  return (
    <div className={styles.app}>
      <div className={styles.asciiColumn}>
        <div className={styles.asciiArtWrapper}>
          <pre className={styles.asciiArtInverted} aria-hidden>{NULLTRACE_ASCII_ART_INVERTED}</pre>
          <pre className={styles.asciiArt}>{NULLTRACE_ASCII_ART}</pre>
        </div>
      </div>
      <div className={styles.infoColumn}>
        <div className={styles.infoLine}><span className={styles.label}>OS</span> nulltrace</div>
        <div className={styles.infoLine}><span className={styles.label}>Version</span> {MOCK_VERSION}</div>
        <div className={styles.infoLine}><span className={styles.label}>Theme</span> {THEME_DISPLAY_NAMES[theme]}</div>
        <div className={styles.infoLine}><span className={styles.label}>CPU</span> {MOCK_CPU}</div>
        <div className={styles.infoLine}><span className={styles.label}>Memory</span> {MOCK_MEMORY}</div>
        <div className={styles.infoLine}><span className={styles.label}>Disk</span> {MOCK_DISK}</div>
        {username && <div className={styles.infoLine}><span className={styles.label}>User</span> {username}</div>}
        <div className={styles.infoLine}><span className={styles.label}>Rank</span> #{MOCK_LEADERBOARD_RANK}</div>
        <div className={styles.infoLine}><span className={styles.label}>Faction</span> {MOCK_FACTION}</div>
      </div>
    </div>
  );
}
