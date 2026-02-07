import { WORLD_SERVER_NAME, VIRTUAL_ROUTER_NAME } from "../lib/networkConfig";
import styles from "./NetworkManager.module.css";

export default function NetworkManager() {
  return (
    <div className={styles.app}>
      <h2 className={styles.title}>Network</h2>
      <p className={styles.subtitle}>
        World server is fixed; virtual router is the network you are connected to.
      </p>
      <div className={styles.rows}>
        <div className={styles.row}>
          <span className={styles.label}>World server</span>
          <span className={styles.value}>{WORLD_SERVER_NAME}</span>
        </div>
        <div className={styles.row}>
          <span className={styles.label}>Virtual router</span>
          <span className={styles.value}>{VIRTUAL_ROUTER_NAME}</span>
        </div>
      </div>
    </div>
  );
}
