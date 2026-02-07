import { useState } from "react";
import styles from "./SoundManager.module.css";

const DEFAULT_VOLUME = 80;

export default function SoundManager() {
  const [master, setMaster] = useState(DEFAULT_VOLUME);
  const [music, setMusic] = useState(DEFAULT_VOLUME);
  const [effects, setEffects] = useState(DEFAULT_VOLUME);
  const [notifications, setNotifications] = useState(DEFAULT_VOLUME);

  return (
    <div className={styles.app}>
      <h2 className={styles.title}>Sound Manager</h2>
      <p className={styles.subtitle}>Adjust volume levels. Master affects all output.</p>
      <div className={styles.sliders}>
        <div className={styles.row}>
          <label className={styles.label} htmlFor="sound-master">
            Master
          </label>
          <div className={styles.sliderWrap}>
            <input
              id="sound-master"
              type="range"
              min={0}
              max={100}
              value={master}
              onChange={(e) => setMaster(Number(e.target.value))}
              className={styles.range}
              style={{ ["--value" as string]: `${master}%` }}
              aria-label="Master volume"
            />
            <span className={styles.value}>{master}%</span>
          </div>
        </div>
        <div className={styles.row}>
          <label className={styles.label} htmlFor="sound-music">
            Music
          </label>
          <div className={styles.sliderWrap}>
            <input
              id="sound-music"
              type="range"
              min={0}
              max={100}
              value={music}
              onChange={(e) => setMusic(Number(e.target.value))}
              className={styles.range}
              style={{ ["--value" as string]: `${music}%` }}
              aria-label="Music volume"
            />
            <span className={styles.value}>{music}%</span>
          </div>
        </div>
        <div className={styles.row}>
          <label className={styles.label} htmlFor="sound-effects">
            Effects
          </label>
          <div className={styles.sliderWrap}>
            <input
              id="sound-effects"
              type="range"
              min={0}
              max={100}
              value={effects}
              onChange={(e) => setEffects(Number(e.target.value))}
              className={styles.range}
              style={{ ["--value" as string]: `${effects}%` }}
              aria-label="Effects volume"
            />
            <span className={styles.value}>{effects}%</span>
          </div>
        </div>
        <div className={styles.row}>
          <label className={styles.label} htmlFor="sound-notifications">
            Notifications
          </label>
          <div className={styles.sliderWrap}>
            <input
              id="sound-notifications"
              type="range"
              min={0}
              max={100}
              value={notifications}
              onChange={(e) => setNotifications(Number(e.target.value))}
              className={styles.range}
              style={{ ["--value" as string]: `${notifications}%` }}
              aria-label="Notifications volume"
            />
            <span className={styles.value}>{notifications}%</span>
          </div>
        </div>
      </div>
    </div>
  );
}
