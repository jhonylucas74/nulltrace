/**
 * Mock terminal command parser. Returns output lines for the simulated shell.
 */
export function runMockCommand(cmd: string, username: string): string[] {
  const trimmed = cmd.trim();
  if (!trimmed) return [];

  const parts = trimmed.split(/\s+/);
  const name = parts[0].toLowerCase();

  switch (name) {
    case "ls": {
      const dirs = ["documents", "hack_tools", "readme.txt", "config"];
      return dirs.map((d) => (d.endsWith("/") || d.includes(".") ? d : d + "/"));
    }
    case "whoami":
      return [username];
    case "help":
      return [
        "Available commands:",
        "  ls         List files",
        "  whoami     Print username",
        "  help       Show this help",
        "  clear      Clear terminal",
        "  neofetch   System info (mock)",
      ];
    case "clear":
      return []; // Caller should clear scrollback when output is []
    case "neofetch": {
      return [
        "                    .-.",
        "                   (   )",
        "                    '-'",
        "  username@nulltrace",
        "  -----------------",
        "  OS: Nulltrace OS (mock)",
        "  Host: nulltrace-client",
        "  Kernel: 6.8.0-nulltrace",
        "  Shell: simulated",
        "  Theme: ricing dark",
      ];
    }
    default:
      return [`Command not found: ${parts[0]}`];
  }
}

export function isClearCommand(cmd: string): boolean {
  return cmd.trim().toLowerCase() === "clear";
}
