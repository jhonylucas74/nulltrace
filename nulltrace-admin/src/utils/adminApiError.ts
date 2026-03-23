/** Extract display message from Tauri invoke error (string or Error). */
export function getAdminApiErrorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  return "Failed to fetch";
}

/** True if the error indicates session expired or invalid admin token. */
export function isSessionExpiredError(msg: string): boolean {
  return (
    msg.includes("ExpiredSignature") ||
    msg.includes("Invalid admin token") ||
    msg.includes("Unauthenticated")
  );
}
