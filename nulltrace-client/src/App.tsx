import { useEffect } from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { useAuth } from "./contexts/AuthContext";
import { getWindowConfigFromStorage } from "./contexts/WindowConfigContext";
import { StartupConfigProvider } from "./contexts/StartupConfigContext";
import { WindowConfigProvider } from "./contexts/WindowConfigContext";
import { ClipboardProvider } from "./contexts/ClipboardContext";
import TokenRefresher from "./components/TokenRefresher";
import Login from "./screens/Login";
import Desktop from "./screens/Desktop";

export default function App() {
  const { username } = useAuth();

  // Apply stored window config on startup when running inside Tauri (fullscreen, start maximized).
  // Runs as soon as the app mounts so the window state is applied before the user sees the first screen.
  useEffect(() => {
    if (typeof window === "undefined") return;
    const tauri = (window as unknown as { __TAURI__?: unknown }).__TAURI__;
    if (!tauri) return;
    const config = getWindowConfigFromStorage();
    const apply = async () => {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const win = getCurrentWindow();
        await win.setFullscreen(config.fullscreen);
        if (config.startMaximized && !config.fullscreen) {
          await win.maximize();
        }
      } catch {
        // Ignore if Tauri window API is unavailable (e.g. in browser).
      }
    };
    const t = setTimeout(apply, 200);
    return () => clearTimeout(t);
  }, []);

  return (
    <>
      {username && <TokenRefresher />}
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route
          path="/desktop"
          element={
            username ? (
              <StartupConfigProvider>
                <WindowConfigProvider>
                  <ClipboardProvider>
                    <Desktop />
                  </ClipboardProvider>
                </WindowConfigProvider>
              </StartupConfigProvider>
            ) : (
              <Navigate to="/login" replace />
            )
          }
        />
        <Route path="/" element={<Navigate to={username ? "/desktop" : "/login"} replace />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </>
  );
}
