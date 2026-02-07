/**
 * Network config: world server (immutable) and virtual router (read-only for now).
 * Can be replaced later by env vars or backend.
 */

/** Immutable: the real server / world the player is on. */
export const WORLD_SERVER_NAME = "nulltrace-1";

/** Virtual router: the network the user is connected to. Read-only for now; may become changeable via game mechanics. */
export const VIRTUAL_ROUTER_NAME = "home";
