import { useWorkspaceLayout } from "../contexts/WorkspaceLayoutContext";
import styles from "./WorkspaceDots.module.css";

export default function WorkspaceDots() {
  const { workspaces, activeWorkspaceId, setActiveWorkspace } = useWorkspaceLayout();

  return (
    <div className={styles.wrap} role="tablist" aria-label="Workspaces">
      {workspaces.map((ws) => (
        <button
          key={ws.id}
          type="button"
          role="tab"
          aria-selected={activeWorkspaceId === ws.id}
          aria-label={ws.label}
          title={ws.label}
          className={styles.dot}
          data-active={activeWorkspaceId === ws.id ? "true" : undefined}
          onClick={() => setActiveWorkspace(ws.id)}
        />
      ))}
    </div>
  );
}
