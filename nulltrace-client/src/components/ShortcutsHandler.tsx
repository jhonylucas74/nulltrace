import { useEffect } from "react";
import { useShortcuts } from "../contexts/ShortcutsContext";
import { useAppLauncher } from "../contexts/AppLauncherContext";
import { useWorkspaceLayout } from "../contexts/WorkspaceLayoutContext";

/**
 * Registers global shortcut handlers with ShortcutsContext.
 * Must be rendered inside ShortcutsProvider, AppLauncherProvider, and WorkspaceLayoutProvider.
 */
export default function ShortcutsHandler() {
  const { registerActionHandler } = useShortcuts();
  const { open: openAppLauncher } = useAppLauncher();
  const { gridModeEnabled, setGridMode, workspaces, activeWorkspaceId, setActiveWorkspace } = useWorkspaceLayout();

  useEffect(() => {
    const goNext = () => {
      const idx = workspaces.findIndex((w) => w.id === activeWorkspaceId);
      const next = workspaces[idx + 1] ?? workspaces[0];
      if (next) setActiveWorkspace(next.id);
    };
    const goPrev = () => {
      const idx = workspaces.findIndex((w) => w.id === activeWorkspaceId);
      const prev = workspaces[idx - 1] ?? workspaces[workspaces.length - 1];
      if (prev) setActiveWorkspace(prev.id);
    };
    const unregApp = registerActionHandler("appLauncher", openAppLauncher);
    const unregGrid = registerActionHandler("toggleGrid", () => setGridMode(!gridModeEnabled));
    const unregNext = registerActionHandler("nextWorkspace", goNext);
    const unregPrev = registerActionHandler("prevWorkspace", goPrev);
    const unregNextAlt = registerActionHandler("nextWorkspaceAlt", goNext);
    const unregPrevAlt = registerActionHandler("prevWorkspaceAlt", goPrev);
    const unregGoTo = [
      registerActionHandler("goToWorkspace1", () => { const w = workspaces[0]; if (w) setActiveWorkspace(w.id); }),
      registerActionHandler("goToWorkspace2", () => { const w = workspaces[1]; if (w) setActiveWorkspace(w.id); }),
      registerActionHandler("goToWorkspace3", () => { const w = workspaces[2]; if (w) setActiveWorkspace(w.id); }),
      registerActionHandler("goToWorkspace4", () => { const w = workspaces[3]; if (w) setActiveWorkspace(w.id); }),
      registerActionHandler("goToWorkspace5", () => { const w = workspaces[4]; if (w) setActiveWorkspace(w.id); }),
      registerActionHandler("goToWorkspace6", () => { const w = workspaces[5]; if (w) setActiveWorkspace(w.id); }),
      registerActionHandler("goToWorkspace7", () => { const w = workspaces[6]; if (w) setActiveWorkspace(w.id); }),
      registerActionHandler("goToWorkspace8", () => { const w = workspaces[7]; if (w) setActiveWorkspace(w.id); }),
      registerActionHandler("goToWorkspace9", () => { const w = workspaces[8]; if (w) setActiveWorkspace(w.id); }),
    ];
    return () => {
      unregApp();
      unregGrid();
      unregNext();
      unregPrev();
      unregNextAlt();
      unregPrevAlt();
      unregGoTo.forEach((u) => u());
    };
  }, [
    registerActionHandler,
    openAppLauncher,
    gridModeEnabled,
    setGridMode,
    workspaces,
    activeWorkspaceId,
    setActiveWorkspace,
  ]);

  return null;
}
