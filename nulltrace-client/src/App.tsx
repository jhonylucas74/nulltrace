import { Routes, Route, Navigate } from "react-router-dom";
import { useAuth } from "./contexts/AuthContext";
import { StartupConfigProvider } from "./contexts/StartupConfigContext";
import Login from "./screens/Login";
import Desktop from "./screens/Desktop";

export default function App() {
  const { username } = useAuth();

  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route
        path="/desktop"
        element={
          username ? (
            <StartupConfigProvider>
              <Desktop />
            </StartupConfigProvider>
          ) : (
            <Navigate to="/login" replace />
          )
        }
      />
      <Route path="/" element={<Navigate to={username ? "/desktop" : "/login"} replace />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
