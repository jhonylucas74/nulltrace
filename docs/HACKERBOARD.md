# Hackerboard — current state and implementation roadmap

This document tracks what the **Hackerboard** in-game app already implements, what remains mock-only or missing, and a suggested **order of work** for future features.

## Summary

| Layer | Done | Not done |
|-------|------|----------|
| **UI shell** | Full layout (feed, rankings, messages, group, profile); **Feed** uses `hackerboard` i18n namespace | Rankings / messages / group / profile still mostly hardcoded English |
| **Rankings & factions (server)** | Cluster gRPC: ranking, create faction, leave faction | Non-cluster server returns stubs only |
| **Social (feed, DMs, group chat)** | **Feed:** Postgres + cluster RPCs (`ListFeedPosts`, `CreateFeedPost`, `ToggleFeedPostLike`), Tauri, client filter + compose language (`en` / `pt-br`) | DMs and faction group chat still local-only; non-cluster **server** stub returns feed errors (use cluster binary) |

---

## Feed — full stack (shipped)

PostgreSQL → cluster gRPC → Tauri → React: shared timeline, reload-safe likes, **per-post language** (`en` | `pt-br`), **server-side list filter** (empty = all languages).

**Security:** identity only from JWT — no `player_id` / `author_id` in gRPC request bodies ([grpc-no-player-id-in-requests](../.cursor/rules/grpc-no-player-id-in-requests.mdc)).

| Piece | Location |
|-------|----------|
| **DB** | `031_create_feed_posts.sql` — `feed_posts`, `feed_post_likes`; wired in `nulltrace-core/src/cluster/db/mod.rs` |
| **Service** | `nulltrace-core/src/cluster/db/feed_service.rs` |
| **Proto** | `game.proto` (client + core); cluster handlers `nulltrace-core/src/cluster/grpc.rs`; stubs `nulltrace-core/src/server/main.rs` |
| **Tauri** | `grpc_list_feed_posts`, `grpc_create_feed_post`, `grpc_toggle_feed_post_like` in `nulltrace-client/src-tauri/src/grpc.rs` |
| **Client** | `HackerboardContext.tsx`, `HackerboardApp.tsx`; i18n `en/hackerboard.json`, `pt-br/hackerboard.json` |

---

## Checklists

Use these boxes to track progress. Checked items are **implemented in the repo today**; unchecked items are **still open**.

### Done

- [x] Hackerboard window type, default size, `HackerboardProvider` / `HackerboardApp` on `Desktop`
- [x] App launcher entry, icons, app name in `apps.json` (pt/en)
- [x] Full UI shell: Feed, Rankings, Messages, Group, profile-related flows
- [x] **Feed (full stack):** `feed_posts` / `feed_post_likes`; `ListFeedPosts` / `CreateFeedPost` / `ToggleFeedPostLike`; per-post language; timeline language filter (default all); Tauri + `HackerboardContext` + feed UI; `hackerboard` i18n for feed tab
- [x] Feed UI: compose, threads, replies, server-backed likes
- [x] Rankings UI: hacker/faction tabs, search, “you” / your faction card
- [x] Messages UI: DM list, pane, faction invite accept/decline (local state only)
- [x] Group UI: faction chat + members (local state only)
- [x] Mock data: hackers, factions (when ranking unavailable), DMs, group messages (feed is API-backed when logged in with cluster)
- [x] **Cluster gRPC `GetRanking`** + **`grpc_get_ranking`** (Tauri) — live ranking when cluster + auth
- [x] **Cluster gRPC `CreateFaction`** + **`grpc_create_faction`** — create faction + assign player
- [x] **Cluster gRPC `LeaveFaction`** + **`grpc_leave_faction`** — clear player faction
- [x] Intentional **exclusion from fixed dock** (open from launcher)

### Not done yet

- [ ] **Direct messages:** server storage and delivery between players
- [ ] **Faction group chat:** shared across clients (server-backed)
- [ ] **Faction social:** invite / accept / join another player’s faction / kick (RPCs + DB); wire UI to server
- [ ] **Gameplay → Hackerboard:** events for hacks/missions → feed rows and/or points (pipeline)
- [ ] **Points:** game systems updating `points` consistently (beyond ranking read)
- [ ] **i18n:** replace remaining hardcoded English in `HackerboardApp` (rankings, messages, group, profile) with namespaces + `en` / `pt-br` JSON
- [ ] **Loading & errors:** explicit UX when ranking fails (vs silent mock fallback); shared patterns
- [ ] **Non-cluster server:** real `GetRanking` / factions / **feed** (today: stub errors — use cluster binary)
- [ ] **Real-time (optional):** push or poll for feed/DMs if needed

### Roadmap phases (unchecked = not started)

**Phase 1 — social core**

- [ ] 1.1 Join / invite faction (server + client wiring)
- [x] 1.2 Persisted feed: DB + list/create/like RPCs + language on posts + server-side language filter
- [x] 1.3 Client: feed + compose/reply + filter backed by API (unauthenticated: empty feed + sign-in prompt)

**Phase 2 — messaging**

- [ ] 2.1 DMs on server
- [ ] 2.2 Faction group chat on server

**Phase 3 — gameplay & polish**

- [ ] 3.1 Events → feed / points
- [ ] 3.2 i18n + empty/error states (can overlap with items above)
- [ ] 3.3 Real-time / scale (optional)

---

## What exists today

### Client (`nulltrace-client`)

- **Window integration:** `WindowType` includes `hackerboard`; default size in `WindowManagerContext`; `Desktop` wraps `HackerboardProvider` and renders `HackerboardApp`.
- **App list / launcher:** Entry with icon and app name keys in `appList` and `apps.json` (pt/en).
- **UI (`HackerboardApp.tsx` + CSS):** Sidebar navigation; **Feed** (compose with language, server-backed list/filter/likes, threads, replies); **Rankings** (hackers vs factions tabs, search, “you” card); **Messages** (DM list, conversation pane, faction invite accept/decline UI); **Group** (faction chat + members); profile flows driven by context.
- **Dock:** Hackerboard is intentionally excluded from the fixed dock (opens from the app launcher).

### State (`HackerboardContext.tsx`)

- **Offline / demo data:** Mock hackers and factions when unauthenticated or when the ranking API does not return usable data; mock DMs and faction group messages.
- **Authenticated + cluster available:**
  - `grpc_get_ranking` → builds hacker list and derived faction aggregates for UI.
  - `grpc_create_faction` / `grpc_leave_faction` → faction membership on the server.
  - **`grpc_list_feed_posts` / `grpc_create_feed_post` / `grpc_toggle_feed_post_like`** → persisted feed with language filter.
- **Still client-only:** DMs, group messages, and faction invite accept/decline **do not** call the backend; invites update React state only.

### Backend (cluster)

- **`GetRanking`:** Authenticated; reads ranking rows and faction names via `player_service` / `faction_service` (`nulltrace-core/src/cluster/grpc.rs`).
- **`CreateFaction` / `LeaveFaction`:** Authenticated; create faction and set/clear `faction_id` on the player (with validation, e.g. cannot create while already in a faction).
- **`ListFeedPosts` / `CreateFeedPost` / `ToggleFeedPostLike`:** Authenticated; `feed_service` + `feed_posts` / `feed_post_likes` tables.

### Tauri bridge

- Commands: `grpc_get_ranking`, `grpc_create_faction`, `grpc_leave_faction`, `grpc_list_feed_posts`, `grpc_create_feed_post`, `grpc_toggle_feed_post_like` (`nulltrace-client/src-tauri/src/grpc.rs`, registered in `lib.rs`).

### Non-cluster server stub

- `nulltrace-core/src/server/main.rs` returns placeholder errors for ranking, faction, and **feed** RPCs (“Use the unified cluster binary…”). Real behavior requires the **unified cluster** binary.

---

## What is missing or incomplete

### Multiplayer / persistence

1. **Feed:** Shipped (server-backed with cluster). **Non-cluster** server binary still returns stub errors for feed RPCs.
2. **Direct messages:** No storage or delivery; conversations are local-only.
3. **Faction group chat:** Local-only; not shared across clients.
4. **Faction membership beyond self-service:** No server RPCs for *invite another player*, *accept invite*, *kick*, or *join existing faction* — the DM “invite” flow is UI-only and does not persist membership.

### Gameplay linkage

5. **“Hacked” / mission posts:** Copy and mock posts describe hacks and missions; there is no pipeline from real game events into feed or point changes.
6. **Points:** Ranking reflects DB points when using cluster; changing points from missions/hacks is a separate systems concern (not owned by Hackerboard UI alone).

### Product / polish

7. **i18n:** Most `HackerboardApp` copy is hardcoded English; extend `pt.json` / `en.json` and wire `useTranslation` (or project equivalent).
8. **Loading and errors:** Clear UX when ranking fails (partially falls back to mock); align with shared loading patterns where applicable.
9. **Documentation for ops:** Ensure dev/prod run the cluster binary for ranking/factions to work end-to-end.

---

## Suggested implementation order

Phases are ordered by **dependency** and **value vs complexity**. Adjust if product priorities differ.

### Phase 1 — Baseline “real” social core (server + client)

| Priority | Item | Rationale |
|----------|------|-----------|
| 1.1 | **Join / invite faction (server)** | Unblocks real teams: RPCs to invite by player id, accept/decline, optional kick/leave rules aligned with DB. Wire Hackerboard DM invite UI to these RPCs. |
| 1.2 | **Persisted feed** | `feed_posts` + `feed_post_likes`; list/create/toggle like; **language** on each post; **list** accepts optional language filter (empty = all). Enables shared timeline across players. |
| 1.3 | **Client: feed backed by API** | Tauri invokes; timeline filter drives `ListFeedPosts`; compose/reply send language; replace mock when logged in with cluster (optional mock when offline). |

### Phase 2 — Messaging

| Priority | Item | Rationale |
|----------|------|-----------|
| 2.1 | **DMs (server)** | Store messages, list conversations, send; auth from JWT only (no `player_id` in bodies — follow project gRPC rules). |
| 2.2 | **Faction group chat (server)** | Room per faction; same auth model. |

### Phase 3 — Gameplay and polish

| Priority | Item | Rationale |
|----------|------|-----------|
| 3.1 | **Events → feed / points** | Emit structured events (mission complete, hack, penalty) to create feed rows and/or adjust points in one place. |
| 3.2 | **i18n + empty/error states** | Consistent copy and accessibility. |
| 3.3 | **Real-time (optional)** | Push or poll for feed/DMs if scale requires it. |

---

## Key files (for implementers)

| Area | Path |
|------|------|
| UI | `nulltrace-client/src/components/HackerboardApp.tsx`, `HackerboardApp.module.css` |
| State + gRPC calls | `nulltrace-client/src/contexts/HackerboardContext.tsx` |
| Tauri | `nulltrace-client/src-tauri/src/grpc.rs`, `lib.rs` |
| Cluster handlers | `nulltrace-core/src/cluster/grpc.rs` |
| Cluster entry | `nulltrace-core/src/cluster/main.rs` |
| Proto | `nulltrace-client/proto/game.proto` (and `nulltrace-core/proto/game.proto`) |
| Ranking / players | `nulltrace-core/src/cluster/db/player_service.rs` |
| Factions | `nulltrace-core` faction service + migrations (e.g. `010_create_factions.sql`) |
| Feed (planned) | Migration `031_create_feed_posts.sql`; `nulltrace-core/src/cluster/db/feed_service.rs` |

---

## Changelog

- **2025-03-24:** **Feed full stack implemented** (migration `031`, `feed_service`, proto + cluster + Tauri + client + `hackerboard` i18n). Doc updated: checklists, “Feed — full stack” section, `What exists today`, stubs note.
- **2025-03-24:** Document updated with **Feed — full-stack plan** (language, filter, DB, gRPC, Tauri, client); summary table and Phase 1 / key files / “Not done yet” aligned with that plan; checklist items 1.2–1.3 clarified.
- **2025-03-24:** Checklists (done / not done / roadmap phases) added.
- **2025-03-24:** Initial roadmap document added.
