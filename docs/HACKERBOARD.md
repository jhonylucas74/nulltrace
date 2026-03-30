# Hackerboard — current state and implementation roadmap

This document tracks what the **Hackerboard** in-game app already implements, what remains mock-only or missing, and a suggested **order of work** for future features.

## Summary

| Layer | Done | Not done |
|-------|------|----------|
| **UI shell** | Full layout (feed, rankings, messages, group, profile); **Feed** uses `hackerboard` i18n namespace | Rankings / messages / group / profile still mostly hardcoded English |
| **Rankings & factions (server)** | Cluster gRPC: ranking (per-viewer, block-filtered; `faction_creator_id` + `faction_allow_member_invites` on entries), create/leave faction, **faction invites** (send by username, list incoming, list outgoing, cancel, accept, decline); `factions.allow_member_invites` (default true; when false, only creator may `SendFactionInvite`); pending invites sent by a player are cancelled when they leave | Non-cluster server returns stubs only |
| **Social (feed, DMs, group chat)** | **Feed** (as above). **Player blocks** (`player_blocks`): symmetric hide in feed, ranking, DM threads, and DM send; gRPC `BlockHackerboardPlayer` / `UnblockHackerboardPlayer` / `ListBlockedPlayers` (`target_username` only; JWT identity). **Faction invites** + **DMs** + **faction group chat:** Postgres + cluster RPCs when `clusterRankingActive`; Tauri + `HackerboardContext`; poll on Messages/Group focus and after send (v1). **Mock** DMs / group chat / offline faction-invite bubbles when ranking is not from the cluster; mock block list in `localStorage` per player | Non-cluster **server** stub returns errors for feed, faction invites, blocks, **and Hackerboard messaging** RPCs (use unified cluster binary) |

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

## Messaging — full stack (Phase 2, shipped)

**DMs:** `target_username` / `peer_username` in requests; **faction chat:** room implied by JWT user’s `players.faction_id` (no `faction_id` in request bodies for authorization).

**Security:** identity only from JWT ([grpc-auth-token](../.cursor/rules/grpc-auth-token.mdc), [grpc-no-player-id-in-requests](../.cursor/rules/grpc-no-player-id-in-requests.mdc)).

| Piece | Location |
|-------|----------|
| **DB** | `034_hackerboard_direct_messages.sql`, `035_hackerboard_faction_chat.sql`; wired in `nulltrace-core/src/cluster/db/mod.rs` |
| **Services** | `hackerboard_dm_service.rs`, `hackerboard_faction_chat_service.rs` |
| **Proto** | `SendHackerboardDm`, `ListHackerboardDmThreads`, `ListHackerboardDmMessages`, `SendHackerboardFactionMessage`, `ListHackerboardFactionMessages` in `game.proto` (client + core) |
| **Cluster** | Handlers in `nulltrace-core/src/cluster/grpc.rs`; stubs in `nulltrace-core/src/server/main.rs` |
| **Tauri** | `grpc_send_hackerboard_dm`, `grpc_list_hackerboard_dm_threads`, `grpc_list_hackerboard_dm_messages`, `grpc_send_hackerboard_faction_message`, `grpc_list_hackerboard_faction_messages` |
| **Client** | `HackerboardContext.tsx` (server state when `token && clusterRankingActive`; `rankingError` + `retryRanking`; `loadOlderDmMessages` / `loadOlderFactionMessages`); `HackerboardApp.tsx` banner when ranking fails, load-older controls, messaging i18n |

---

## Checklists

Use these boxes to track progress. Checked items are **implemented in the repo today**; unchecked items are **still open**.

### Done

- [x] Hackerboard window type, default size, `HackerboardProvider` / `HackerboardApp` on `Desktop`
- [x] App launcher entry, icons, app name in `apps.json` (pt/en)
- [x] Full UI shell: Feed, Rankings, Messages, Group, profile-related flows
- [x] **Feed (full stack):** `feed_posts` / `feed_post_likes`; `ListFeedPosts` / `CreateFeedPost` / `ToggleFeedPostLike`; per-post language; timeline language filter (default all); Tauri + `HackerboardContext` + feed UI; `hackerboard` i18n for feed tab
- [x] Feed UI: compose, threads, replies, server-backed likes
- [x] **Feed ready (v1):** infinite scroll / keyset pagination, top-idle and manual refresh, optimistic likes, report post with local hide, persisted feed filter + compose language (DB), replies visible under language filter, language UI (icon menus)
- [x] Rankings UI: hacker/faction tabs, search, “you” / your faction card
- [x] Messages UI: DM list, pane; mock faction invite bubbles offline; **cluster** faction invites inbox + Accept/Decline (server)
- [x] Group UI: faction chat + members; **cluster:** server-backed group chat when in a faction and live ranking is active
- [x] Mock data: hackers, factions (when ranking unavailable), DMs, group messages (feed is API-backed when logged in with cluster)
- [x] **Cluster gRPC `GetRanking`** + **`grpc_get_ranking`** (Tauri) — live ranking when cluster + auth
- [x] **Cluster gRPC `CreateFaction`** + **`grpc_create_faction`** — create faction + assign player
- [x] **Cluster gRPC `LeaveFaction`** + **`grpc_leave_faction`** — clear player faction; cancels pending invites **sent by** the leaver
- [x] **Faction invites (1.1):** `faction_invites` migration; `SendFactionInvite` / `ListFactionInvites` / `AcceptFactionInvite` / `DeclineFactionInvite` / `ListOutgoingFactionInvites` / `CancelFactionInvite`; `037` `factions.allow_member_invites`; Tauri commands; Hackerboard Messages inbox + group invite-by-username + outgoing list; profile “invite to my faction”; i18n keys for invites
- [x] **Player blocks:** `036_player_blocks`; feed/ranking/DM filtering; `BlockHackerboardPlayer` / `UnblockHackerboardPlayer` / `ListBlockedPlayers`; Tauri; context + profile / DM header / client feed filter for defense-in-depth
- [x] **DMs (2.1):** `hackerboard_dm_messages`; send / list threads / list messages RPCs; Tauri; Messages tab uses server when `clusterRankingActive`
- [x] **Faction group chat (2.2):** `hackerboard_faction_messages`; send / list RPCs; Tauri; Group chat uses server when `clusterRankingActive` and user has a faction
- [x] Intentional **exclusion from fixed dock** (open from launcher)

### Not done yet

- [ ] **Faction social:** kick member (and optional “request join” without invite) — invite/accept/join is **server-backed** (see 1.1)
- [ ] **Gameplay → Hackerboard:** events for hacks/missions → feed rows and/or points (pipeline)
- [ ] **Points:** game systems updating `points` consistently (beyond ranking read)
- [ ] **i18n:** replace remaining hardcoded English in `HackerboardApp` (rankings, messages, group, profile) with namespaces + `en` / `pt-br` JSON
- [ ] **Loading & errors:** explicit UX when ranking fails (vs silent mock fallback); shared patterns
- [ ] **Non-cluster server:** real `GetRanking` / factions / **feed** (today: stub errors — use cluster binary)
- [ ] **Real-time (optional):** push or poll for feed/DMs if needed

### Roadmap phases (unchecked = not started)

**Phase 1 — social core**

- [x] 1.1 Join / invite faction (server + client wiring)
- [x] 1.2 Persisted feed: DB + list/create/like RPCs + language on posts + server-side language filter
- [x] 1.3 Client: feed + compose/reply + filter backed by API (unauthenticated: empty feed + sign-in prompt)

**Phase 2 — messaging**

- [x] 2.1 DMs on server
- [x] 2.2 Faction group chat on server

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

- **Offline / demo data:** Mock hackers and factions when unauthenticated or when the ranking API does not return usable data; mock DMs, faction group messages, and mock DM-thread faction invites when the cluster ranking path is inactive.
- **Authenticated + cluster available (`clusterRankingActive`):**
  - `grpc_get_ranking` → builds hacker list and derived faction aggregates for UI.
  - `grpc_create_faction` / `grpc_leave_faction` → faction membership on the server.
  - **`grpc_send_faction_invite` / `grpc_list_faction_invites` / `grpc_accept_faction_invite` / `grpc_decline_faction_invite`** → persisted invites; inbox in Messages when live ranking is loaded.
  - **`grpc_list_feed_posts` / `grpc_create_feed_post` / `grpc_toggle_feed_post_like`** → persisted feed with language filter.
  - **`grpc_send_hackerboard_dm` / `grpc_list_hackerboard_dm_threads` / `grpc_list_hackerboard_dm_messages`** → persisted DMs (refresh on Messages / peer select / after send; **Load older messages** uses `before_message_id`; ~60s polling while Messages or Group is open).
  - **`grpc_send_hackerboard_faction_message` / `grpc_list_hackerboard_faction_messages`** → persisted faction group chat (same refresh / pagination / polling pattern).
- **Still client-only when ranking is not live:** mock DMs, mock group chat, mock faction-invite bubbles (no server persistence for those paths).

### Backend (cluster)

- **`GetRanking`:** Authenticated; reads ranking rows and faction names via `player_service` / `faction_service` (`nulltrace-core/src/cluster/grpc.rs`).
- **`CreateFaction` / `LeaveFaction`:** Authenticated; create faction and set/clear `faction_id` on the player (with validation, e.g. cannot create while already in a faction). `LeaveFaction` cancels pending invites where `from_player_id` is the leaver.
- **`SendFactionInvite` / `ListFactionInvites` / `AcceptFactionInvite` / `DeclineFactionInvite`:** Authenticated; `faction_invite_service` + `faction_invites` table (identity from JWT only).
- **`ListFeedPosts` / `CreateFeedPost` / `ToggleFeedPostLike`:** Authenticated; `feed_service` + `feed_posts` / `feed_post_likes` tables.
- **`SendHackerboardDm` / `ListHackerboardDmThreads` / `ListHackerboardDmMessages`:** Authenticated; `hackerboard_dm_service` + `hackerboard_dm_messages`.
- **`SendHackerboardFactionMessage` / `ListHackerboardFactionMessages`:** Authenticated; `hackerboard_faction_chat_service` + `hackerboard_faction_messages`.

### Tauri bridge

- Commands: `grpc_get_ranking`, `grpc_create_faction`, `grpc_leave_faction`, `grpc_send_faction_invite`, `grpc_list_faction_invites`, `grpc_accept_faction_invite`, `grpc_decline_faction_invite`, `grpc_list_feed_posts`, `grpc_create_feed_post`, `grpc_toggle_feed_post_like`, `grpc_send_hackerboard_dm`, `grpc_list_hackerboard_dm_threads`, `grpc_list_hackerboard_dm_messages`, `grpc_send_hackerboard_faction_message`, `grpc_list_hackerboard_faction_messages` (`nulltrace-client/src-tauri/src/grpc.rs`, registered in `lib.rs`).

### Non-cluster server stub

- `nulltrace-core/src/server/main.rs` returns placeholder errors for ranking, faction (including faction invites), **feed**, and **Hackerboard messaging** RPCs (“Use the unified cluster binary…”). Real behavior requires the **unified cluster** binary.

---

## What is missing or incomplete

### Multiplayer / persistence

1. **Feed:** Shipped (server-backed with cluster). **Non-cluster** server binary still returns stub errors for feed RPCs.
2. **Direct messages / faction group chat:** Shipped on the **cluster** (see **Messaging — full stack**). Local-only when ranking is not from the cluster.
3. **Faction moderation:** No server RPC for *kick member* (and optional “request join” without invite) yet.

### Gameplay linkage

4. **“Hacked” / mission posts:** Copy and mock posts describe hacks and missions; there is no pipeline from real game events into feed or point changes.
5. **Points:** Ranking reflects DB points when using cluster; changing points from missions/hacks is a separate systems concern (not owned by Hackerboard UI alone).

### Product / polish

6. **i18n:** Most `HackerboardApp` copy is still hardcoded English outside the feed and new messaging keys; extend `hackerboard` JSON and wire `useTranslation`.
7. **Loading and errors:** Ranking failures show a top **banner** with localized copy and **Retry** (mock data remains usable). Further polish (e.g. in-flight ranking spinner) can align with shared loading patterns.
8. **Documentation for ops:** Ensure dev/prod run the cluster binary for ranking/factions/messaging to work end-to-end.

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
| Feed | Migration `031_create_feed_posts.sql`; `nulltrace-core/src/cluster/db/feed_service.rs` |
| Hackerboard DMs | Migration `034_hackerboard_direct_messages.sql`; `hackerboard_dm_service.rs` |
| Hackerboard faction chat | Migration `035_hackerboard_faction_chat.sql`; `hackerboard_faction_chat_service.rs` |

---

## Changelog

- **2026-03-27 (post–Phase 2):** Client: DM and faction chat **older-message pagination** (`before_message_id`); **ranking error** state + banner + retry; **60s polling** on Messages/Group when cluster ranking is active; i18n keys `rankingClusterRequired`, `rankingNetworkError`, `retryRanking`, `loadOlderMessages`, `loadingOlderMessages`.
- **2026-03-27:** **Phase 2 messaging:** DMs + faction group chat (migrations `034`–`035`, services, proto, cluster gRPC, server stubs, Tauri, `HackerboardContext` / `HackerboardApp`, polling refresh, messaging i18n). Doc: summary, checklists, “What exists today”, Tauri list, stubs, key files, “missing” section.
- **2025-03-24:** **Feed full stack implemented** (migration `031`, `feed_service`, proto + cluster + Tauri + client + `hackerboard` i18n). Doc updated: checklists, “Feed — full stack” section, `What exists today`, stubs note.
- **2025-03-24:** Document updated with **Feed — full-stack plan** (language, filter, DB, gRPC, Tauri, client); summary table and Phase 1 / key files / “Not done yet” aligned with that plan; checklist items 1.2–1.3 clarified.
- **2025-03-24:** Checklists (done / not done / roadmap phases) added.
- **2025-03-24:** Initial roadmap document added.
