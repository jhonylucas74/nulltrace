import { useState } from "react";
import { LayoutDashboard, Server, Network, LogOut } from "lucide-react";
import { useAuth } from "../contexts/AuthContext";
import { useNavigate } from "react-router-dom";
import DashboardOverview from "../components/DashboardOverview";
import VmTable from "../components/VmTable";
import NetworkTopology from "../components/NetworkTopology";
import styles from "./Dashboard.module.css";

type TabId = "overview" | "vms" | "network";

export default function Dashboard() {
  const { token, logout } = useAuth();
  const navigate = useNavigate();
  const [tab, setTab] = useState<TabId>("overview");

  function handleLogout() {
    logout();
    navigate("/login", { replace: true });
  }

  const tabs: { id: TabId; label: string; icon: React.ReactNode }[] = [
    { id: "overview", label: "Dashboard", icon: <LayoutDashboard size={18} /> },
    { id: "vms", label: "VMs", icon: <Server size={18} /> },
    { id: "network", label: "Network", icon: <Network size={18} /> },
  ];

  return (
    <div className={styles.layout}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarHeader}>
          <h2 className={styles.sidebarTitle}>Nulltrace Admin</h2>
        </div>
        <nav className={styles.nav}>
          {tabs.map((t) => (
            <button
              key={t.id}
              className={`${styles.navItem} ${tab === t.id ? styles.navItemActive : ""}`}
              onClick={() => setTab(t.id)}
            >
              {t.icon}
              <span>{t.label}</span>
            </button>
          ))}
        </nav>
        <div className={styles.sidebarFooter}>
          <button className={styles.logoutBtn} onClick={handleLogout}>
            <LogOut size={18} />
            <span>Sign out</span>
          </button>
        </div>
      </aside>
      <main className={styles.main}>
        {tab === "overview" && token && <DashboardOverview token={token} />}
        {tab === "vms" && token && <VmTable token={token} />}
        {tab === "network" && token && <NetworkTopology token={token} />}
      </main>
    </div>
  );
}
