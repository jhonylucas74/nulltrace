import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useAuth, parseJwt } from "../contexts/AuthContext";
import { useGrpc } from "../contexts/GrpcContext";

/**
 * TokenRefresher - Background component that automatically refreshes JWT tokens
 * before they expire (2 hours before expiry).
 *
 * - Checks every 5 minutes if token needs refresh
 * - Refreshes when < 2 hours remaining
 * - On failure, logs out and redirects to login
 */
export default function TokenRefresher() {
  const { token, tokenExpiresAt, login, logout, username, playerId } = useAuth();
  const { refreshToken } = useGrpc();
  const navigate = useNavigate();

  useEffect(() => {
    if (!token || !tokenExpiresAt || !username || !playerId) return;

    const checkInterval = setInterval(async () => {
      const now = Date.now() / 1000;
      const twoHours = 2 * 60 * 60;

      // Check if token expires in less than 2 hours
      if (tokenExpiresAt - now < twoHours) {
        try {
          console.log("[TokenRefresher] Token expiring soon, refreshing...");
          const result = await refreshToken(token);
          if (result.success && result.token) {
            const claims = parseJwt(result.token);
            login(username, playerId, result.token, claims.exp);
            console.log("[TokenRefresher] Token refreshed successfully");
          } else {
            console.error("[TokenRefresher] Token refresh failed:", result.error_message);
            logout();
            navigate("/login");
          }
        } catch (err) {
          console.error("[TokenRefresher] Token refresh error:", err);
          logout();
          navigate("/login");
        }
      }
    }, 5 * 60 * 1000); // Check every 5 minutes

    return () => clearInterval(checkInterval);
  }, [token, tokenExpiresAt, username, playerId, login, logout, refreshToken, navigate]);

  // This component renders nothing
  return null;
}
