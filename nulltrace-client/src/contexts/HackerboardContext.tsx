import React, { createContext, useContext, useMemo, useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { useAuth } from "./AuthContext";

export interface Hacker {
  id: string;
  username: string;
  points: number;
  factionId: string | null;
}

export interface Faction {
  id: string;
  name: string;
  memberIds: string[];
  /** Faction creator player id; used for invite permission UI. */
  creatorId: string | null;
  /** When false, only `creatorId` may send invites (server-enforced). */
  allowMemberInvites: boolean;
}

export type FeedPostType = "hacked" | "mission" | "system" | "user";

export type FeedPostLanguage = "en" | "pt-br";

export interface FeedPost {
  id: string;
  type: FeedPostType;
  body: string;
  timestamp: number;
  hackerId?: string;
  targetId?: string;
  authorId?: string;
  replyToId?: string;
  likeCount?: number;
  /** Present for server-backed and new user posts */
  language?: FeedPostLanguage;
}

export type DMMessageType = "text" | "faction_invite";

export interface DMMessage {
  id: string;
  senderId: string;
  body: string;
  timestamp: number;
  type?: DMMessageType;
  factionId?: string;
  factionName?: string;
  invitedByUserId?: string;
  accepted?: boolean;
}

/** Pending invite from ListFactionInvites (cluster). */
export interface ServerFactionInvite {
  inviteId: string;
  factionId: string;
  factionName: string;
  fromUsername: string;
  createdAtMs: number;
}

/** Pending invite sent by the viewer's faction (cluster). */
export interface ServerFactionInviteOutgoing {
  inviteId: string;
  toUsername: string;
  fromUsername: string;
  fromPlayerId: string;
  createdAtMs: number;
}

export interface FactionGroupMessage {
  id: string;
  factionId: string;
  senderId: string;
  body: string;
  timestamp: number;
}

export interface DmConversationItem {
  conversationId: string;
  otherParticipantId: string;
}

/** Mock hackers with points and optional faction. */
const MOCK_HACKERS: Hacker[] = [
  { id: "h1", username: "neon_cipher", points: 2840, factionId: "f1" },
  { id: "h2", username: "void_runner", points: 2520, factionId: "f1" },
  { id: "h3", username: "byte_bandit", points: 2310, factionId: "f2" },
  { id: "h4", username: "root_access", points: 2100, factionId: null },
  { id: "h5", username: "shadow_stack", points: 1980, factionId: "f2" },
  { id: "h6", username: "null_ptr", points: 1750, factionId: "f1" },
  { id: "h7", username: "shell_shock", points: 1620, factionId: "f3" },
  { id: "h8", username: "hex_driver", points: 1480, factionId: null },
  { id: "h9", username: "kernel_panic", points: 1350, factionId: "f3" },
  { id: "h10", username: "hacker", points: 1200, factionId: "f2" },
  { id: "h11", username: "stack_overflow", points: 980, factionId: "f1" },
  { id: "h12", username: "buffer_queen", points: 820, factionId: null },
  { id: "h13", username: "sigkill", points: 650, factionId: "f3" },
  { id: "h14", username: "daemon_lord", points: 490, factionId: "f2" },
  { id: "h15", username: "fork_bomb", points: 310, factionId: null },
];

/** Mock factions. */
/** Page size for cluster-backed Hackerboard DM and faction chat lists. */
const HACKERBOARD_DM_PAGE_LIMIT = 50;
const HACKERBOARD_FACTION_PAGE_LIMIT = 50;

const MOCK_FACTIONS: Faction[] = [
  {
    id: "f1",
    name: "Zero Day Collective",
    memberIds: ["h1", "h2", "h6", "h11"],
    creatorId: "h1",
    allowMemberInvites: true,
  },
  {
    id: "f2",
    name: "Null Protocol",
    memberIds: ["h3", "h5", "h10", "h14"],
    creatorId: "h3",
    allowMemberInvites: true,
  },
  {
    id: "f3",
    name: "Deep Signal",
    memberIds: ["h7", "h9", "h13"],
    creatorId: "h7",
    allowMemberInvites: true,
  },
];

function mapApiEntryToFeedPost(p: {
  id: string;
  author_id: string;
  author_username: string;
  body: string;
  language: string;
  created_at_ms: number;
  reply_to_id: string;
  post_type: string;
  like_count: number;
  liked_by_me: boolean;
}): FeedPost {
  const pt = p.post_type as FeedPostType;
  const type: FeedPostType =
    pt === "system" || pt === "hacked" || pt === "mission" || pt === "user" ? pt : "user";
  const lang = p.language === "pt-br" || p.language === "en" ? p.language : undefined;
  return {
    id: p.id,
    type,
    body: p.body,
    timestamp: p.created_at_ms,
    authorId: p.author_id,
    replyToId: p.reply_to_id || undefined,
    likeCount: p.like_count,
    language: lang,
  };
}

/** Page size for ListFeedPosts (server max 100). */
const FEED_PAGE_SIZE = 25;
/** Cap merged + infinite-scroll feed length in memory (oldest trimmed after merge). */
const FEED_MAX_IN_MEMORY = 300;

type FeedListRow = Parameters<typeof mapApiEntryToFeedPost>[0];

/** Canonical conversation id for a pair of hackers. */
export function getConversationId(userId1: string, userId2: string): string {
  return `dm-${[userId1, userId2].sort().join("-")}`;
}

function mockBlockedStorageKey(playerId: string): string {
  return `nulltrace.hackerboard.blockedIds.${playerId}`;
}

function readMockBlockedIds(playerId: string): Set<string> {
  try {
    const raw = localStorage.getItem(mockBlockedStorageKey(playerId));
    if (!raw) return new Set();
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return new Set();
    return new Set(parsed.filter((x): x is string => typeof x === "string"));
  } catch {
    return new Set();
  }
}

function writeMockBlockedIds(playerId: string, ids: Set<string>) {
  try {
    localStorage.setItem(mockBlockedStorageKey(playerId), JSON.stringify([...ids]));
  } catch {
    /* ignore */
  }
}

/** Build initial DM conversations (h10 = hacker default user with a few threads). */
function buildInitialDmConversations(): Record<string, DMMessage[]> {
  const ts = Date.now() - 3600 * 1000 * 24;
  const step = 1000 * 60 * 5;
  const conv1 = getConversationId("h10", "h7");
  const conv2 = getConversationId("h10", "h3");
  const conv3 = getConversationId("h10", "h1");
  return {
    [conv1]: [
      { id: "dm1-1", senderId: "h7", body: "Hey, still in for the weekend op?", timestamp: ts },
      { id: "dm1-2", senderId: "h10", body: "Yes, count me in.", timestamp: ts + step },
      { id: "dm1-3", senderId: "h7", body: "Great. We'll meet at 20:00.", timestamp: ts + step * 2 },
    ],
    [conv2]: [
      { id: "dm2-1", senderId: "h3", body: "Raid tonight - you joining?", timestamp: ts + step * 3 },
      { id: "dm2-2", senderId: "h10", body: "Yeah, I'll be there.", timestamp: ts + step * 4 },
    ],
    [conv3]: [
      { id: "dm3-1", senderId: "h1", body: "Check the new exploit, might help with the mainframe.", timestamp: ts + step * 5 },
      { id: "dm3-2", senderId: "h10", body: "On it, thanks.", timestamp: ts + step * 6 },
    ],
  };
}

/** Build initial faction group messages per faction. */
function buildInitialFactionGroupMessages(): Record<string, FactionGroupMessage[]> {
  const ts = Date.now() - 3600 * 1000 * 12;
  const step = 1000 * 60 * 8;
  return {
    f1: [
      { id: "fg1-1", factionId: "f1", senderId: "h1", body: "Everyone ready for the push?", timestamp: ts },
      { id: "fg1-2", factionId: "f1", senderId: "h2", body: "Ready.", timestamp: ts + step },
      { id: "fg1-3", factionId: "f1", senderId: "h6", body: "Same here.", timestamp: ts + step * 2 },
    ],
    f2: [
      { id: "fg2-1", factionId: "f2", senderId: "h3", body: "Raid in 1h. Be online.", timestamp: ts },
      { id: "fg2-2", factionId: "f2", senderId: "h5", body: "Will do.", timestamp: ts + step },
      { id: "fg2-3", factionId: "f2", senderId: "h10", body: "I'm in.", timestamp: ts + step * 2 },
    ],
    f3: [
      { id: "fg3-1", factionId: "f3", senderId: "h7", body: "Weekend op details in the doc.", timestamp: ts },
      { id: "fg3-2", factionId: "f3", senderId: "h9", body: "Got it.", timestamp: ts + step },
    ],
  };
}

export interface HackerWithRank extends Hacker {
  rank: number;
}

export interface FactionWithRank extends Faction {
  totalPoints: number;
  rank: number;
}

export type FeedLanguageFilter = "all" | FeedPostLanguage;

function i18nLangToComposeDefault(lng: string): FeedPostLanguage {
  return lng === "pt-br" || lng.startsWith("pt") ? "pt-br" : "en";
}

interface HackerboardContextValue {
  hackers: HackerWithRank[];
  factions: FactionWithRank[];
  feed: FeedPost[];
  userLikedPostIds: Set<string>;
  feedLanguageFilter: FeedLanguageFilter;
  setFeedLanguageFilter: (f: FeedLanguageFilter) => void;
  composePostLanguage: FeedPostLanguage;
  setComposePostLanguage: (l: FeedPostLanguage) => void;
  feedLoading: boolean;
  feedLoadingMore: boolean;
  feedRefreshing: boolean;
  feedHasMore: boolean;
  feedError: string | null;
  /** True when ranking was loaded from the cluster (live Hackerboard backend). */
  clusterRankingActive: boolean;
  /** Set when logged in but `grpc_get_ranking` failed; UI shows mock ranking. Empty when OK or not logged in. */
  rankingError: string | null;
  /** Bump to re-fetch ranking (e.g. after "Retry"). */
  retryRanking: () => void;
  refreshFeed: () => void;
  loadMoreFeed: () => Promise<void>;
  searchHackers: (query: string) => HackerWithRank[];
  searchFactions: (query: string) => FactionWithRank[];
  addFeedPost: (post: Omit<FeedPost, "id" | "timestamp"> & { language: FeedPostLanguage }) => Promise<void>;
  toggleLike: (postId: string, currentlyLiked: boolean) => Promise<void>;
  getDmConversations: (currentUserId: string) => DmConversationItem[];
  getDmMessages: (conversationId: string) => DMMessage[];
  sendDm: (
    senderId: string,
    otherParticipantId: string,
    body: string
  ) => Promise<{ success: boolean; errorMessage?: string }>;
  /** Reload server-backed DM threads and messages for the open peer (cluster only). */
  refreshDmConversation: (currentUserId: string, otherParticipantId: string) => Promise<void>;
  /** Reload DM threads + faction chat from the cluster (e.g. when opening Messages / Group). */
  refreshHackerboardMessaging: () => Promise<void>;
  /** Whether more older DM messages can be loaded for this conversation (cluster path). */
  hasMoreOlderDmMessages: (conversationId: string) => boolean;
  loadOlderDmMessages: (currentUserId: string, otherParticipantId: string) => Promise<void>;
  hasMoreOlderFactionMessages: (factionId: string) => boolean;
  loadOlderFactionMessages: (factionId: string) => Promise<void>;
  getFactionGroupMessages: (factionId: string) => FactionGroupMessage[];
  sendFactionGroupMessage: (
    factionId: string,
    senderId: string,
    body: string
  ) => Promise<{ success: boolean; errorMessage?: string }>;
  getEffectiveFactionId: (userId: string) => string | null;
  createFaction: (name: string, creatorUserId: string) => Promise<Faction | null>;
  leaveFaction: (userId: string) => void | Promise<void>;
  factionInvitesIncoming: ServerFactionInvite[];
  sendFactionInvite: (
    fromUserId: string,
    toUserId: string,
    factionId: string
  ) => Promise<{ success: boolean; errorMessage?: string }>;
  acceptFactionInvite: (messageId: string, acceptingUserId: string) => void;
  declineFactionInvite: (messageId: string) => void;
  acceptServerFactionInvite: (inviteId: string) => Promise<{ success: boolean; errorMessage?: string }>;
  declineServerFactionInvite: (inviteId: string) => Promise<{ success: boolean; errorMessage?: string }>;
  /** Player ids the current user has blocked (cluster list or mock localStorage). */
  blockedPlayerIds: ReadonlySet<string>;
  isBlockedByMe: (playerId: string) => boolean;
  blockPlayer: (targetUsername: string) => Promise<{ success: boolean; errorMessage?: string }>;
  unblockPlayer: (targetUsername: string) => Promise<{ success: boolean; errorMessage?: string }>;
  /** Whether the viewer may invite `targetPlayerId` to their faction (cluster rules + mock). */
  canSendFactionInvite: (viewerPlayerId: string, targetPlayerId: string) => boolean;
  sendFactionInviteByUsername: (
    username: string
  ) => Promise<{ success: boolean; errorMessage?: string }>;
  factionInvitesOutgoing: ServerFactionInviteOutgoing[];
  refreshFactionInvitesOutgoing: () => Promise<void>;
  cancelFactionInviteOutgoing: (
    inviteId: string
  ) => Promise<{ success: boolean; errorMessage?: string }>;
}

const HackerboardContext = createContext<HackerboardContextValue | null>(null);

function computeHackersWithRank(hackers: Hacker[]): HackerWithRank[] {
  const sorted = [...hackers].sort((a, b) => b.points - a.points);
  return sorted.map((h, i) => ({ ...h, rank: i + 1 }));
}

function computeFactionsWithRank(hackers: Hacker[], factions: Faction[]): FactionWithRank[] {
  const byId = new Map(hackers.map((h) => [h.id, h]));
  const withTotal = factions.map((f) => {
    const totalPoints = f.memberIds.reduce((sum, id) => sum + (byId.get(id)?.points ?? 0), 0);
    return { ...f, totalPoints };
  });
  const sorted = [...withTotal].sort((a, b) => b.totalPoints - a.totalPoints);
  return sorted.map((f, i) => ({ ...f, rank: i + 1 }));
}

export function HackerboardProvider({ children }: { children: React.ReactNode }) {
  const { token, playerId } = useAuth();
  const { i18n } = useTranslation("hackerboard");
  const [feed, setFeed] = useState<FeedPost[]>([]);
  const [feedLanguageFilter, setFeedLanguageFilterState] = useState<FeedLanguageFilter>("all");
  const [composePostLanguage, setComposePostLanguageState] = useState<FeedPostLanguage>(() =>
    i18nLangToComposeDefault(i18n.language)
  );
  const persistTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const feedFilterRef = useRef<FeedLanguageFilter>(feedLanguageFilter);
  const composeRef = useRef<FeedPostLanguage>(composePostLanguage);
  feedFilterRef.current = feedLanguageFilter;
  composeRef.current = composePostLanguage;
  const [feedLoading, setFeedLoading] = useState(false);
  const [feedLoadingMore, setFeedLoadingMore] = useState(false);
  const [feedRefreshing, setFeedRefreshing] = useState(false);
  const feedRefreshingRef = useRef(false);
  const [feedHasMore, setFeedHasMore] = useState(true);
  const [feedError, setFeedError] = useState<string | null>(null);
  const [userLikedPostIds, setUserLikedPostIds] = useState<Set<string>>(new Set());
  const [dmConversations, setDmConversations] = useState<Record<string, DMMessage[]>>(buildInitialDmConversations);
  const [factionGroupMessages, setFactionGroupMessages] = useState<Record<string, FactionGroupMessage[]>>(
    buildInitialFactionGroupMessages
  );
  const [factions, setFactions] = useState<Faction[]>(() => [...MOCK_FACTIONS]);
  const [membershipOverlay, setMembershipOverlay] = useState<Record<string, string | null>>({});

  const [apiRanking, setApiRanking] = useState<{ hackers: Hacker[]; factions: Faction[] } | null>(null);
  const [rankingError, setRankingError] = useState<string | null>(null);
  const [rankingRefreshTrigger, setRankingRefreshTrigger] = useState(0);
  const [factionInvitesIncoming, setFactionInvitesIncoming] = useState<ServerFactionInvite[]>([]);
  const [factionInvitesOutgoing, setFactionInvitesOutgoing] = useState<ServerFactionInviteOutgoing[]>([]);
  const [blockedPlayerIds, setBlockedPlayerIds] = useState<Set<string>>(() => new Set());
  const [serverDmThreads, setServerDmThreads] = useState<
    { peerPlayerId: string; peerUsername: string; lastCreatedAtMs: number }[]
  >([]);
  const [serverDmMessages, setServerDmMessages] = useState<Record<string, DMMessage[]>>({});
  const [serverFactionMessages, setServerFactionMessages] = useState<Record<string, FactionGroupMessage[]>>({});
  const [serverDmHasOlder, setServerDmHasOlder] = useState<Record<string, boolean>>({});
  const [serverFactionHasOlder, setServerFactionHasOlder] = useState<Record<string, boolean>>({});

  const serverDmMessagesRef = useRef<Record<string, DMMessage[]>>({});
  serverDmMessagesRef.current = serverDmMessages;
  const serverDmHasOlderRef = useRef<Record<string, boolean>>({});
  serverDmHasOlderRef.current = serverDmHasOlder;
  const serverFactionMessagesRef = useRef<Record<string, FactionGroupMessage[]>>({});
  serverFactionMessagesRef.current = serverFactionMessages;
  const serverFactionHasOlderRef = useRef<Record<string, boolean>>({});
  serverFactionHasOlderRef.current = serverFactionHasOlder;

  const fetchRanking = useCallback(async () => {
    if (!token) {
      setApiRanking(null);
      setRankingError(null);
      return;
    }
    setRankingError(null);
    try {
      const res = await invoke<{
        entries: Array<{
          rank: number;
          player_id: string;
          username: string;
          points: number;
          faction_id: string;
          faction_name: string;
          faction_creator_id: string;
          faction_allow_member_invites: boolean;
        }>;
        error_message: string;
      }>("grpc_get_ranking", { token });
      if (res.error_message) {
        setApiRanking(null);
        setRankingError(res.error_message);
        return;
      }
      const hackersFromApi: Hacker[] = res.entries.map((e) => ({
        id: e.player_id,
        username: e.username,
        points: e.points,
        factionId: e.faction_id || null,
      }));
      const factionMap = new Map<
        string,
        { name: string; memberIds: string[]; creatorId: string; allowMemberInvites: boolean }
      >();
      for (const e of res.entries) {
        if (!e.faction_id) continue;
        const cur = factionMap.get(e.faction_id);
        const creatorId = (e.faction_creator_id ?? "").trim();
        const allowMemberInvites = e.faction_allow_member_invites !== false;
        if (cur) {
          cur.memberIds.push(e.player_id);
          if (!cur.creatorId && creatorId) cur.creatorId = creatorId;
        } else {
          factionMap.set(e.faction_id, {
            name: e.faction_name || "Faction",
            memberIds: [e.player_id],
            creatorId,
            allowMemberInvites,
          });
        }
      }
      const factionsFromApi: Faction[] = Array.from(factionMap.entries()).map(([id, v]) => ({
        id,
        name: v.name,
        memberIds: v.memberIds,
        creatorId: v.creatorId || null,
        allowMemberInvites: v.allowMemberInvites,
      }));
      setApiRanking({ hackers: hackersFromApi, factions: factionsFromApi });
      setRankingError(null);
    } catch {
      setApiRanking(null);
      setRankingError("__ranking_network__");
    }
  }, [token]);

  const retryRanking = useCallback(() => {
    setRankingRefreshTrigger((n) => n + 1);
  }, []);

  useEffect(() => {
    fetchRanking();
  }, [fetchRanking, rankingRefreshTrigger]);

  const loadFactionInvites = useCallback(async () => {
    if (!token) {
      setFactionInvitesIncoming([]);
      return;
    }
    try {
      const res = await invoke<{
        invites: Array<{
          invite_id: string;
          faction_id: string;
          faction_name: string;
          from_username: string;
          created_at_ms: number;
        }>;
        error_message: string;
      }>("grpc_list_faction_invites", { token });
      if (res.error_message) {
        setFactionInvitesIncoming([]);
        return;
      }
      setFactionInvitesIncoming(
        res.invites.map((i) => ({
          inviteId: i.invite_id,
          factionId: i.faction_id,
          factionName: i.faction_name,
          fromUsername: i.from_username,
          createdAtMs: i.created_at_ms,
        }))
      );
    } catch {
      setFactionInvitesIncoming([]);
    }
  }, [token]);

  useEffect(() => {
    void loadFactionInvites();
  }, [token, rankingRefreshTrigger, loadFactionInvites]);

  const loadBlockedPlayers = useCallback(async () => {
    if (!playerId) {
      setBlockedPlayerIds(new Set());
      return;
    }
    if (token && apiRanking) {
      try {
        const res = await invoke<{
          blocked: Array<{ player_id: string; username: string }>;
          error_message: string;
        }>("grpc_list_blocked_players", { token });
        if (res.error_message) {
          setBlockedPlayerIds(new Set());
          return;
        }
        setBlockedPlayerIds(new Set(res.blocked.map((b) => b.player_id)));
      } catch {
        setBlockedPlayerIds(new Set());
      }
      return;
    }
    if (token) {
      setBlockedPlayerIds(readMockBlockedIds(playerId));
    } else {
      setBlockedPlayerIds(new Set());
    }
  }, [token, apiRanking, playerId]);

  useEffect(() => {
    void loadBlockedPlayers();
  }, [loadBlockedPlayers, rankingRefreshTrigger]);

  const loadFactionInvitesOutgoing = useCallback(async () => {
    if (!token || !apiRanking) {
      setFactionInvitesOutgoing([]);
      return;
    }
    try {
      const res = await invoke<{
        invites: Array<{
          invite_id: string;
          to_username: string;
          from_username: string;
          from_player_id: string;
          created_at_ms: number;
        }>;
        error_message: string;
      }>("grpc_list_outgoing_faction_invites", { token });
      if (res.error_message) {
        setFactionInvitesOutgoing([]);
        return;
      }
      setFactionInvitesOutgoing(
        res.invites.map((i) => ({
          inviteId: i.invite_id,
          toUsername: i.to_username,
          fromUsername: i.from_username,
          fromPlayerId: i.from_player_id,
          createdAtMs: i.created_at_ms,
        }))
      );
    } catch {
      setFactionInvitesOutgoing([]);
    }
  }, [token, apiRanking]);

  useEffect(() => {
    void loadFactionInvitesOutgoing();
  }, [loadFactionInvitesOutgoing, rankingRefreshTrigger]);

  const loadDmThreads = useCallback(async () => {
    if (!token || !apiRanking) {
      setServerDmThreads([]);
      return;
    }
    try {
      const res = await invoke<{
        threads: Array<{
          peer_player_id: string;
          peer_username: string;
          last_message_id: string;
          last_body: string;
          last_created_at_ms: number;
        }>;
        error_message: string;
      }>("grpc_list_hackerboard_dm_threads", { token, limit: 50 });
      if (res.error_message) {
        setServerDmThreads([]);
        return;
      }
      setServerDmThreads(
        res.threads.map((t) => ({
          peerPlayerId: t.peer_player_id,
          peerUsername: t.peer_username,
          lastCreatedAtMs: t.last_created_at_ms,
        }))
      );
    } catch {
      setServerDmThreads([]);
    }
  }, [token, apiRanking]);

  const loadDmMessagesForPair = useCallback(
    async (currentUserId: string, otherParticipantId: string) => {
      if (!token || !apiRanking) return;
      const other = apiRanking.hackers.find((h) => h.id === otherParticipantId);
      if (!other) return;
      const cid = getConversationId(currentUserId, otherParticipantId);
      try {
        const res = await invoke<{
          messages: Array<{
            id: string;
            from_player_id: string;
            body: string;
            created_at_ms: number;
          }>;
          error_message: string;
        }>("grpc_list_hackerboard_dm_messages", {
          token,
          peerUsername: other.username,
          beforeMessageId: "",
          limit: HACKERBOARD_DM_PAGE_LIMIT,
        });
        if (res.error_message) return;
        const mapped: DMMessage[] = res.messages.map((m) => ({
          id: m.id,
          senderId: m.from_player_id,
          body: m.body,
          timestamp: m.created_at_ms,
          type: "text" as const,
        }));
        setServerDmMessages((prev) => ({ ...prev, [cid]: mapped }));
        setServerDmHasOlder((prev) => ({
          ...prev,
          [cid]: mapped.length >= HACKERBOARD_DM_PAGE_LIMIT,
        }));
      } catch {
        /* ignore */
      }
    },
    [token, apiRanking]
  );

  const loadOlderDmMessages = useCallback(
    async (currentUserId: string, otherParticipantId: string) => {
      if (!token || !apiRanking) return;
      const other = apiRanking.hackers.find((h) => h.id === otherParticipantId);
      if (!other) return;
      const cid = getConversationId(currentUserId, otherParticipantId);
      if (!serverDmHasOlderRef.current[cid]) return;
      const existing = serverDmMessagesRef.current[cid] ?? [];
      if (existing.length === 0) return;
      const oldestId = existing[0].id;
      try {
        const res = await invoke<{
          messages: Array<{
            id: string;
            from_player_id: string;
            body: string;
            created_at_ms: number;
          }>;
          error_message: string;
        }>("grpc_list_hackerboard_dm_messages", {
          token,
          peerUsername: other.username,
          beforeMessageId: oldestId,
          limit: HACKERBOARD_DM_PAGE_LIMIT,
        });
        if (res.error_message) return;
        const mapped: DMMessage[] = res.messages.map((m) => ({
          id: m.id,
          senderId: m.from_player_id,
          body: m.body,
          timestamp: m.created_at_ms,
          type: "text" as const,
        }));
        if (mapped.length === 0) {
          setServerDmHasOlder((prev) => ({ ...prev, [cid]: false }));
          return;
        }
        setServerDmMessages((prev) => ({
          ...prev,
          [cid]: [...mapped, ...(prev[cid] ?? [])],
        }));
        setServerDmHasOlder((prev) => ({
          ...prev,
          [cid]: mapped.length >= HACKERBOARD_DM_PAGE_LIMIT,
        }));
      } catch {
        /* ignore */
      }
    },
    [token, apiRanking]
  );

  const loadFactionRoomMessages = useCallback(
    async (factionId: string) => {
      if (!token) return;
      try {
        const res = await invoke<{
          messages: Array<{
            id: string;
            from_player_id: string;
            from_username: string;
            body: string;
            created_at_ms: number;
          }>;
          error_message: string;
        }>("grpc_list_hackerboard_faction_messages", {
          token,
          beforeMessageId: "",
          limit: HACKERBOARD_FACTION_PAGE_LIMIT,
        });
        if (res.error_message) return;
        const mapped: FactionGroupMessage[] = res.messages.map((m) => ({
          id: m.id,
          factionId,
          senderId: m.from_player_id,
          body: m.body,
          timestamp: m.created_at_ms,
        }));
        setServerFactionMessages((prev) => ({ ...prev, [factionId]: mapped }));
        setServerFactionHasOlder((prev) => ({
          ...prev,
          [factionId]: mapped.length >= HACKERBOARD_FACTION_PAGE_LIMIT,
        }));
      } catch {
        /* ignore */
      }
    },
    [token]
  );

  const loadOlderFactionMessages = useCallback(
    async (factionId: string) => {
      if (!token) return;
      if (!serverFactionHasOlderRef.current[factionId]) return;
      const existing = serverFactionMessagesRef.current[factionId] ?? [];
      if (existing.length === 0) return;
      const oldestId = existing[0].id;
      try {
        const res = await invoke<{
          messages: Array<{
            id: string;
            from_player_id: string;
            from_username: string;
            body: string;
            created_at_ms: number;
          }>;
          error_message: string;
        }>("grpc_list_hackerboard_faction_messages", {
          token,
          beforeMessageId: oldestId,
          limit: HACKERBOARD_FACTION_PAGE_LIMIT,
        });
        if (res.error_message) return;
        const mapped: FactionGroupMessage[] = res.messages.map((m) => ({
          id: m.id,
          factionId,
          senderId: m.from_player_id,
          body: m.body,
          timestamp: m.created_at_ms,
        }));
        if (mapped.length === 0) {
          setServerFactionHasOlder((prev) => ({ ...prev, [factionId]: false }));
          return;
        }
        setServerFactionMessages((prev) => ({
          ...prev,
          [factionId]: [...mapped, ...(prev[factionId] ?? [])],
        }));
        setServerFactionHasOlder((prev) => ({
          ...prev,
          [factionId]: mapped.length >= HACKERBOARD_FACTION_PAGE_LIMIT,
        }));
      } catch {
        /* ignore */
      }
    },
    [token]
  );

  const refreshDmConversation = useCallback(
    async (currentUserId: string, otherParticipantId: string) => {
      if (!token || !apiRanking) return;
      await loadDmMessagesForPair(currentUserId, otherParticipantId);
      await loadDmThreads();
    },
    [token, apiRanking, loadDmMessagesForPair, loadDmThreads]
  );

  const refreshHackerboardMessaging = useCallback(async () => {
    await loadDmThreads();
    if (!token || !apiRanking || !playerId) return;
    const me = apiRanking.hackers.find((h) => h.id === playerId);
    if (me?.factionId) {
      await loadFactionRoomMessages(me.factionId);
    }
  }, [token, apiRanking, playerId, loadDmThreads, loadFactionRoomMessages]);

  useEffect(() => {
    if (!token || !apiRanking) {
      setServerDmThreads([]);
      setServerDmMessages({});
      setServerDmHasOlder({});
      return;
    }
    void loadDmThreads();
  }, [token, apiRanking, rankingRefreshTrigger, loadDmThreads]);

  useEffect(() => {
    if (!token || !apiRanking || !playerId) {
      setServerFactionMessages({});
      setServerFactionHasOlder({});
      return;
    }
    const me = apiRanking.hackers.find((h) => h.id === playerId);
    if (!me?.factionId) {
      setServerFactionMessages({});
      setServerFactionHasOlder({});
      return;
    }
    void loadFactionRoomMessages(me.factionId);
  }, [token, apiRanking, playerId, rankingRefreshTrigger, loadFactionRoomMessages]);

  const schedulePersistHackerboardPrefs = useCallback(() => {
    if (!token) return;
    if (persistTimerRef.current) clearTimeout(persistTimerRef.current);
    persistTimerRef.current = setTimeout(() => {
      persistTimerRef.current = null;
      const ff = feedFilterRef.current;
      const pl = composeRef.current;
      const feedStr = ff === "all" ? "all" : ff;
      void invoke<{ success: boolean; error_message: string }>("grpc_set_hackerboard_language_preferences", {
        token,
        feedLanguageFilter: feedStr,
        postLanguage: pl,
      }).catch(() => {});
    }, 300);
  }, [token]);

  const setFeedLanguageFilter = useCallback(
    (f: FeedLanguageFilter) => {
      feedFilterRef.current = f;
      setFeedLanguageFilterState(f);
      schedulePersistHackerboardPrefs();
    },
    [schedulePersistHackerboardPrefs]
  );

  const setComposePostLanguage = useCallback(
    (l: FeedPostLanguage) => {
      composeRef.current = l;
      setComposePostLanguageState(l);
      schedulePersistHackerboardPrefs();
    },
    [schedulePersistHackerboardPrefs]
  );

  useEffect(() => {
    if (!token) {
      setFeedLanguageFilterState("all");
      setComposePostLanguageState(i18nLangToComposeDefault(i18n.language));
      setFeedHasMore(true);
      setFeedLoadingMore(false);
    }
  }, [token, i18n.language]);

  useEffect(() => {
    if (!token) return;
    let cancelled = false;
    void (async () => {
      try {
        const res = await invoke<{
          error_message: string;
          hackerboard_feed_language_filter?: string;
          hackerboard_post_language?: string;
        }>("grpc_get_player_profile", { token });
        if (cancelled || res.error_message) return;
        const ff = (res.hackerboard_feed_language_filter ?? "").trim();
        if (ff === "all" || ff === "en" || ff === "pt-br") {
          const next: FeedLanguageFilter = ff === "all" ? "all" : (ff as FeedPostLanguage);
          feedFilterRef.current = next;
          setFeedLanguageFilterState(next);
        }
        const pl = (res.hackerboard_post_language ?? "").trim();
        if (pl === "en" || pl === "pt-br") {
          composeRef.current = pl;
          setComposePostLanguageState(pl);
        }
      } catch {
        /* ignore */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [token]);

  const fetchFeedPage = useCallback(
    async (beforePostId: string) => {
      if (!token) {
        throw new Error("Not authenticated");
      }
      const language_filter = feedLanguageFilter === "all" ? "" : feedLanguageFilter;
      return invoke<{ posts: FeedListRow[]; error_message: string }>("grpc_list_feed_posts", {
        token,
        languageFilter: language_filter,
        limit: FEED_PAGE_SIZE,
        beforePostId,
      });
    },
    [token, feedLanguageFilter]
  );

  const loadFeedInitial = useCallback(async () => {
    if (!token) {
      setFeed([]);
      setUserLikedPostIds(new Set());
      setFeedError(null);
      setFeedLoading(false);
      setFeedLoadingMore(false);
      setFeedHasMore(true);
      return;
    }
    setFeedLoading(true);
    setFeedError(null);
    try {
      const res = await fetchFeedPage("");
      if (res.error_message) {
        setFeed([]);
        setUserLikedPostIds(new Set());
        setFeedError(res.error_message);
        setFeedHasMore(false);
        return;
      }
      const mapped = res.posts.map((p) => mapApiEntryToFeedPost(p));
      setFeed(mapped);
      const liked = new Set<string>();
      for (const p of res.posts) {
        if (p.liked_by_me) liked.add(p.id);
      }
      setUserLikedPostIds(liked);
      setFeedHasMore(res.posts.length === FEED_PAGE_SIZE);
    } catch {
      setFeed([]);
      setUserLikedPostIds(new Set());
      setFeedError("Failed to load feed");
      setFeedHasMore(false);
    } finally {
      setFeedLoading(false);
    }
  }, [token, fetchFeedPage]);

  const loadMoreFeed = useCallback(async () => {
    if (!token || !feedHasMore || feedLoadingMore || feedLoading) return;
    const oldestId = feed[feed.length - 1]?.id;
    if (!oldestId) return;
    setFeedLoadingMore(true);
    try {
      const res = await fetchFeedPage(oldestId);
      if (res.error_message) {
        return;
      }
      const mapped = res.posts.map((p) => mapApiEntryToFeedPost(p));
      setFeed((prev) => {
        const ids = new Set(prev.map((p) => p.id));
        const extra = mapped.filter((p) => !ids.has(p.id));
        return [...prev, ...extra];
      });
      setUserLikedPostIds((prev) => {
        const next = new Set(prev);
        for (const p of res.posts) {
          if (p.liked_by_me) next.add(p.id);
          else next.delete(p.id);
        }
        return next;
      });
      setFeedHasMore(res.posts.length === FEED_PAGE_SIZE);
    } catch {
      /* ignore */
    } finally {
      setFeedLoadingMore(false);
    }
  }, [token, feedHasMore, feedLoadingMore, feedLoading, feed, fetchFeedPage]);

  const refreshFeedMerge = useCallback(async () => {
    if (!token) return;
    if (feedRefreshingRef.current) return;
    feedRefreshingRef.current = true;
    setFeedRefreshing(true);
    const startedAt = Date.now();
    const MIN_REFRESH_MS = 450;
    const endRefreshing = () => {
      const wait = Math.max(0, MIN_REFRESH_MS - (Date.now() - startedAt));
      if (wait > 0) {
        window.setTimeout(() => {
          feedRefreshingRef.current = false;
          setFeedRefreshing(false);
        }, wait);
      } else {
        feedRefreshingRef.current = false;
        setFeedRefreshing(false);
      }
    };
    try {
      const res = await fetchFeedPage("");
      if (res.error_message) {
        setFeedError(res.error_message);
        return;
      }
      setFeedError(null);
      setFeed((prev) => {
        const map = new Map(prev.map((p) => [p.id, p]));
        for (const row of res.posts) {
          map.set(row.id, mapApiEntryToFeedPost(row));
        }
        let arr = Array.from(map.values()).sort((a, b) => b.timestamp - a.timestamp);
        if (arr.length > FEED_MAX_IN_MEMORY) {
          arr = arr.slice(0, FEED_MAX_IN_MEMORY);
        }
        return arr;
      });
      setUserLikedPostIds((prev) => {
        const next = new Set(prev);
        for (const p of res.posts) {
          if (p.liked_by_me) next.add(p.id);
          else next.delete(p.id);
        }
        return next;
      });
    } catch {
      setFeedError("Failed to load feed");
    } finally {
      endRefreshing();
    }
  }, [token, fetchFeedPage]);

  const refreshFeed = useCallback(() => {
    void refreshFeedMerge();
  }, [refreshFeedMerge]);

  useEffect(() => {
    void loadFeedInitial();
  }, [loadFeedInitial]);

  const hackers = useMemo(() => {
    const list = apiRanking?.hackers ?? MOCK_HACKERS;
    return computeHackersWithRank(list);
  }, [apiRanking]);

  const factionsWithRank = useMemo(() => {
    if (apiRanking) {
      return computeFactionsWithRank(apiRanking.hackers, apiRanking.factions);
    }
    return computeFactionsWithRank(MOCK_HACKERS, factions);
  }, [apiRanking, factions]);

  const getEffectiveFactionId = useCallback(
    (userId: string): string | null => {
      if (userId in membershipOverlay) return membershipOverlay[userId];
      const list = apiRanking?.hackers ?? MOCK_HACKERS;
      return list.find((h) => h.id === userId)?.factionId ?? null;
    },
    [membershipOverlay, apiRanking]
  );

  const createFaction = useCallback(
    async (name: string, creatorUserId: string): Promise<Faction | null> => {
      if (token && apiRanking) {
        try {
          const res = await invoke<{ faction_id: string; name: string; error_message: string }>(
            "grpc_create_faction",
            { name: name.trim(), token }
          );
          if (res.error_message) return null;
          setRankingRefreshTrigger((t) => t + 1);
          return {
            id: res.faction_id,
            name: res.name,
            memberIds: [creatorUserId],
            creatorId: creatorUserId,
            allowMemberInvites: true,
          };
        } catch {
          return null;
        }
      }
      const id = `f-${Date.now()}`;
      const faction: Faction = {
        id,
        name,
        memberIds: [creatorUserId],
        creatorId: creatorUserId,
        allowMemberInvites: true,
      };
      setFactions((prev) => [...prev, faction]);
      setMembershipOverlay((prev) => ({ ...prev, [creatorUserId]: id }));
      return faction;
    },
    [token, apiRanking]
  );

  const leaveFaction = useCallback(
    async (userId: string) => {
      if (token && apiRanking) {
        try {
          await invoke<{ success: boolean; error_message: string }>("grpc_leave_faction", { token });
          setRankingRefreshTrigger((t) => t + 1);
        } catch {
          // ignore
        }
        return;
      }
      const currentFactionId = membershipOverlay[userId] ?? MOCK_HACKERS.find((h) => h.id === userId)?.factionId ?? null;
      if (!currentFactionId) return;
      setFactions((prev) =>
        prev.map((f) => (f.id === currentFactionId ? { ...f, memberIds: f.memberIds.filter((id) => id !== userId) } : f))
      );
      setMembershipOverlay((prev) => ({ ...prev, [userId]: null }));
    },
    [token, apiRanking, membershipOverlay]
  );

  const sendFactionInvite = useCallback(
    async (
      fromUserId: string,
      toUserId: string,
      factionId: string
    ): Promise<{ success: boolean; errorMessage?: string }> => {
      const hackersList = apiRanking?.hackers ?? MOCK_HACKERS;
      const target = hackersList.find((h) => h.id === toUserId);
      const targetUsername = target?.username;
      if (!targetUsername) {
        return { success: false, errorMessage: "Player not found" };
      }

      if (token && apiRanking) {
        try {
          const res = await invoke<{ invite_id: string; error_message: string }>("grpc_send_faction_invite", {
            targetUsername,
            token,
          });
          if (res.error_message) {
            return { success: false, errorMessage: res.error_message };
          }
          void loadFactionInvites();
          void loadFactionInvitesOutgoing();
          return { success: true };
        } catch {
          return { success: false, errorMessage: "Failed to send invite" };
        }
      }

      const faction = factions.find((f) => f.id === factionId);
      const factionName = faction?.name ?? "Faction";
      const conversationId = getConversationId(fromUserId, toUserId);
      const message: DMMessage = {
        id: `dm-invite-${Date.now()}`,
        senderId: fromUserId,
        body: `You've been invited to join ${factionName}.`,
        timestamp: Date.now(),
        type: "faction_invite",
        factionId,
        factionName,
        invitedByUserId: fromUserId,
      };
      setDmConversations((prev) => ({
        ...prev,
        [conversationId]: [...(prev[conversationId] ?? []), message],
      }));
      return { success: true };
    },
    [token, apiRanking, factions, loadFactionInvites, loadFactionInvitesOutgoing]
  );

  const acceptServerFactionInvite = useCallback(
    async (inviteId: string): Promise<{ success: boolean; errorMessage?: string }> => {
      if (!token || !apiRanking) {
        return { success: false, errorMessage: "Not available offline" };
      }
      try {
        const res = await invoke<{ success: boolean; error_message: string }>("grpc_accept_faction_invite", {
          inviteId,
          token,
        });
        if (!res.success) {
          return { success: false, errorMessage: res.error_message };
        }
        setRankingRefreshTrigger((t) => t + 1);
        void loadFactionInvites();
        return { success: true };
      } catch {
        return { success: false, errorMessage: "Failed to accept invite" };
      }
    },
    [token, apiRanking, loadFactionInvites]
  );

  const declineServerFactionInvite = useCallback(
    async (inviteId: string): Promise<{ success: boolean; errorMessage?: string }> => {
      if (!token || !apiRanking) {
        return { success: false, errorMessage: "Not available offline" };
      }
      try {
        const res = await invoke<{ success: boolean; error_message: string }>("grpc_decline_faction_invite", {
          inviteId,
          token,
        });
        if (!res.success) {
          return { success: false, errorMessage: res.error_message };
        }
        void loadFactionInvites();
        return { success: true };
      } catch {
        return { success: false, errorMessage: "Failed to decline invite" };
      }
    },
    [token, apiRanking, loadFactionInvites]
  );

  const isBlockedByMe = useCallback((pid: string) => blockedPlayerIds.has(pid), [blockedPlayerIds]);

  const blockPlayer = useCallback(
    async (targetUsername: string): Promise<{ success: boolean; errorMessage?: string }> => {
      const u = targetUsername.trim();
      if (!u) return { success: false, errorMessage: "Username is required" };
      if (token && apiRanking) {
        try {
          const res = await invoke<{ error_message: string }>("grpc_block_hackerboard_player", {
            targetUsername: u,
            token,
          });
          if (res.error_message) return { success: false, errorMessage: res.error_message };
          await loadBlockedPlayers();
          return { success: true };
        } catch {
          return { success: false, errorMessage: "__network__" };
        }
      }
      if (!playerId) return { success: false, errorMessage: "Not signed in" };
      const target = MOCK_HACKERS.find((h) => h.username.toLowerCase() === u.toLowerCase());
      if (!target) return { success: false, errorMessage: "Player not found" };
      if (target.id === playerId) return { success: false, errorMessage: "Cannot block yourself" };
      setBlockedPlayerIds((prev) => {
        const next = new Set(prev);
        next.add(target.id);
        writeMockBlockedIds(playerId, next);
        return next;
      });
      return { success: true };
    },
    [token, apiRanking, playerId, loadBlockedPlayers]
  );

  const unblockPlayer = useCallback(
    async (targetUsername: string): Promise<{ success: boolean; errorMessage?: string }> => {
      const u = targetUsername.trim();
      if (!u) return { success: false, errorMessage: "Username is required" };
      if (token && apiRanking) {
        try {
          const res = await invoke<{ error_message: string }>("grpc_unblock_hackerboard_player", {
            targetUsername: u,
            token,
          });
          if (res.error_message) return { success: false, errorMessage: res.error_message };
          await loadBlockedPlayers();
          return { success: true };
        } catch {
          return { success: false, errorMessage: "__network__" };
        }
      }
      if (!playerId) return { success: false, errorMessage: "Not signed in" };
      const target = MOCK_HACKERS.find((h) => h.username.toLowerCase() === u.toLowerCase());
      if (!target) return { success: false, errorMessage: "Player not found" };
      setBlockedPlayerIds((prev) => {
        const next = new Set(prev);
        next.delete(target.id);
        writeMockBlockedIds(playerId, next);
        return next;
      });
      return { success: true };
    },
    [token, apiRanking, playerId, loadBlockedPlayers]
  );

  const canSendFactionInvite = useCallback(
    (viewerPlayerId: string, targetPlayerId: string): boolean => {
      if (viewerPlayerId === targetPlayerId) return false;
      if (blockedPlayerIds.has(targetPlayerId)) return false;
      const hackersList = apiRanking?.hackers ?? MOCK_HACKERS;
      const target = hackersList.find((h) => h.id === targetPlayerId);
      if (!target || target.factionId) return false;
      const fid = getEffectiveFactionId(viewerPlayerId);
      if (!fid) return false;
      const facList = apiRanking?.factions ?? factions;
      const fac = facList.find((f) => f.id === fid);
      if (!fac || !fac.memberIds.includes(viewerPlayerId)) return false;
      if (!fac.allowMemberInvites) return fac.creatorId === viewerPlayerId;
      return true;
    },
    [apiRanking, factions, getEffectiveFactionId, blockedPlayerIds]
  );

  const sendFactionInviteByUsername = useCallback(
    async (username: string): Promise<{ success: boolean; errorMessage?: string }> => {
      const u = username.trim();
      if (!u) return { success: false, errorMessage: "__invite_username_empty__" };
      if (token && apiRanking) {
        try {
          const res = await invoke<{ invite_id: string; error_message: string }>("grpc_send_faction_invite", {
            targetUsername: u,
            token,
          });
          if (res.error_message) return { success: false, errorMessage: res.error_message };
          void loadFactionInvites();
          void loadFactionInvitesOutgoing();
          return { success: true };
        } catch {
          return { success: false, errorMessage: "__network__" };
        }
      }
      if (!playerId) return { success: false, errorMessage: "Not signed in" };
      const target = MOCK_HACKERS.find((h) => h.username.toLowerCase() === u.toLowerCase());
      if (!target) return { success: false, errorMessage: "Player not found" };
      const fid = getEffectiveFactionId(playerId);
      if (!fid) return { success: false, errorMessage: "You are not in a faction" };
      return sendFactionInvite(playerId, target.id, fid);
    },
    [
      token,
      apiRanking,
      playerId,
      getEffectiveFactionId,
      loadFactionInvites,
      loadFactionInvitesOutgoing,
      sendFactionInvite,
    ]
  );

  const cancelFactionInviteOutgoing = useCallback(
    async (inviteId: string): Promise<{ success: boolean; errorMessage?: string }> => {
      if (!token || !apiRanking) {
        return { success: false, errorMessage: "Not available offline" };
      }
      try {
        const res = await invoke<{ success: boolean; error_message: string }>("grpc_cancel_faction_invite", {
          inviteId,
          token,
        });
        if (!res.success) return { success: false, errorMessage: res.error_message };
        void loadFactionInvitesOutgoing();
        return { success: true };
      } catch {
        return { success: false, errorMessage: "__network__" };
      }
    },
    [token, apiRanking, loadFactionInvitesOutgoing]
  );

  const acceptFactionInvite = useCallback(
    (messageId: string, acceptingUserId: string) => {
      let conversationId: string | null = null;
      let foundFactionId: string | undefined;
      for (const [cid, msgs] of Object.entries(dmConversations)) {
        const m = msgs.find((x) => x.id === messageId);
        if (m && m.type === "faction_invite" && m.factionId) {
          const parts = cid.replace(/^dm-/, "").split("-");
          const other = parts[0] === m.senderId ? parts[1] : parts[0];
          if (other === acceptingUserId) {
            conversationId = cid;
            foundFactionId = m.factionId;
            break;
          }
        }
      }
      if (!conversationId || !foundFactionId) return;
      const factionId = foundFactionId;
      setFactions((prev) =>
        prev.map((f) =>
          f.id === factionId
            ? { ...f, memberIds: f.memberIds.includes(acceptingUserId) ? f.memberIds : [...f.memberIds, acceptingUserId] }
            : f
        )
      );
      setMembershipOverlay((prev) => ({ ...prev, [acceptingUserId]: factionId }));
      setDmConversations((prev) => ({
        ...prev,
        [conversationId]: prev[conversationId].map((m) => (m.id === messageId ? { ...m, accepted: true } : m)),
      }));
    },
    [dmConversations]
  );

  const declineFactionInvite = useCallback((messageId: string) => {
    setDmConversations((prev) => {
      const next = { ...prev };
      Object.keys(next).forEach((cid) => {
        next[cid] = next[cid].map((m) => (m.id === messageId ? { ...m, accepted: false } : m));
      });
      return next;
    });
  }, []);

  const addFeedPost = useCallback(
    async (post: Omit<FeedPost, "id" | "timestamp"> & { language: FeedPostLanguage }) => {
      if (!token) return;
      const res = await invoke<{ post: FeedListRow | null; error_message: string }>("grpc_create_feed_post", {
        token,
        body: post.body,
        language: post.language,
        replyToPostId: post.replyToId ?? "",
      });
      if (res.error_message) {
        throw new Error(res.error_message);
      }
      if (res.post) {
        const mapped = mapApiEntryToFeedPost(res.post);
        setFeed((prev) => {
          const byId = new Map(prev.map((p) => [p.id, p]));
          byId.set(mapped.id, mapped);
          let arr = Array.from(byId.values()).sort((a, b) => b.timestamp - a.timestamp);
          if (arr.length > FEED_MAX_IN_MEMORY) {
            arr = arr.slice(0, FEED_MAX_IN_MEMORY);
          }
          return arr;
        });
        setUserLikedPostIds((prev) => {
          const next = new Set(prev);
          if (res.post!.liked_by_me) next.add(res.post!.id);
          return next;
        });
        setFeedError(null);
        return;
      }
      await loadFeedInitial();
    },
    [token, loadFeedInitial]
  );

  const toggleLike = useCallback(async (postId: string, currentlyLiked: boolean) => {
    if (!token) return;

    const wasLiked = currentlyLiked;
    setUserLikedPostIds((prev) => {
      const next = new Set(prev);
      if (wasLiked) next.delete(postId);
      else next.add(postId);
      return next;
    });
    setFeed((prev) =>
      prev.map((p) => {
        if (p.id !== postId) return p;
        const count = p.likeCount ?? 0;
        return { ...p, likeCount: Math.max(0, wasLiked ? count - 1 : count + 1) };
      })
    );

    try {
      const res = await invoke<{ liked: boolean; like_count: number; error_message: string }>(
        "grpc_toggle_feed_post_like",
        { token, postId }
      );
      if (res.error_message) {
        throw new Error(res.error_message);
      }
      setFeed((prev) =>
        prev.map((p) => (p.id === postId ? { ...p, likeCount: res.like_count } : p))
      );
      setUserLikedPostIds((prev) => {
        const next = new Set(prev);
        if (res.liked) next.add(postId);
        else next.delete(postId);
        return next;
      });
    } catch {
      setUserLikedPostIds((prev) => {
        const next = new Set(prev);
        if (wasLiked) next.add(postId);
        else next.delete(postId);
        return next;
      });
      setFeed((prev) =>
        prev.map((p) => {
          if (p.id !== postId) return p;
          const count = p.likeCount ?? 0;
          return { ...p, likeCount: Math.max(0, wasLiked ? count + 1 : count - 1) };
        })
      );
    }
  }, [token]);

  const getDmConversations = useCallback(
    (currentUserId: string): DmConversationItem[] => {
      if (token && apiRanking) {
        return [...serverDmThreads]
          .filter((t) => !blockedPlayerIds.has(t.peerPlayerId))
          .sort((a, b) => b.lastCreatedAtMs - a.lastCreatedAtMs)
          .map((t) => ({
            conversationId: getConversationId(currentUserId, t.peerPlayerId),
            otherParticipantId: t.peerPlayerId,
          }));
      }
      const items: DmConversationItem[] = [];
      Object.keys(dmConversations).forEach((conversationId) => {
        const parts = conversationId.replace(/^dm-/, "").split("-");
        if (parts.length >= 2) {
          if (parts[0] === currentUserId) {
            items.push({ conversationId, otherParticipantId: parts[1] });
          } else if (parts[1] === currentUserId) {
            items.push({ conversationId, otherParticipantId: parts[0] });
          }
        }
      });
      return items
        .filter((it) => !blockedPlayerIds.has(it.otherParticipantId))
        .sort((a, b) => {
          const msgsA = dmConversations[a.conversationId] ?? [];
          const msgsB = dmConversations[b.conversationId] ?? [];
          const lastA = msgsA[msgsA.length - 1]?.timestamp ?? 0;
          const lastB = msgsB[msgsB.length - 1]?.timestamp ?? 0;
          return lastB - lastA;
        });
    },
    [token, apiRanking, serverDmThreads, dmConversations, blockedPlayerIds]
  );

  const getDmMessages = useCallback(
    (conversationId: string): DMMessage[] => {
      if (token && apiRanking) {
        const msgs = serverDmMessages[conversationId] ?? [];
        return [...msgs].sort((a, b) => a.timestamp - b.timestamp);
      }
      const msgs = dmConversations[conversationId] ?? [];
      return [...msgs].sort((a, b) => a.timestamp - b.timestamp);
    },
    [token, apiRanking, serverDmMessages, dmConversations]
  );

  const sendDm = useCallback(
    async (
      senderId: string,
      otherParticipantId: string,
      body: string
    ): Promise<{ success: boolean; errorMessage?: string }> => {
      if (token && apiRanking) {
        const other = apiRanking.hackers.find((h) => h.id === otherParticipantId);
        if (!other) return { success: false, errorMessage: "__peer_unavailable__" };
        if (blockedPlayerIds.has(otherParticipantId)) {
          return { success: false, errorMessage: "__dm_blocked__" };
        }
        try {
          const res = await invoke<{ message_id: string; error_message: string }>("grpc_send_hackerboard_dm", {
            token,
            targetUsername: other.username,
            body: body.trim(),
          });
          if (res.error_message) return { success: false, errorMessage: res.error_message };
        } catch {
          return { success: false, errorMessage: "__network__" };
        }
        await refreshDmConversation(senderId, otherParticipantId);
        return { success: true };
      }
      if (blockedPlayerIds.has(otherParticipantId)) {
        return { success: false, errorMessage: "__dm_blocked__" };
      }
      const conversationId = getConversationId(senderId, otherParticipantId);
      const message: DMMessage = {
        id: `dm-msg-${Date.now()}`,
        senderId,
        body,
        timestamp: Date.now(),
        type: "text",
      };
      setDmConversations((prev) => ({
        ...prev,
        [conversationId]: [...(prev[conversationId] ?? []), message],
      }));
      return { success: true };
    },
    [token, apiRanking, refreshDmConversation, blockedPlayerIds]
  );

  const getFactionGroupMessages = useCallback(
    (factionId: string): FactionGroupMessage[] => {
      if (token && apiRanking) {
        const msgs = serverFactionMessages[factionId] ?? [];
        return [...msgs].sort((a, b) => a.timestamp - b.timestamp);
      }
      const msgs = factionGroupMessages[factionId] ?? [];
      return [...msgs].sort((a, b) => a.timestamp - b.timestamp);
    },
    [token, apiRanking, serverFactionMessages, factionGroupMessages]
  );

  const sendFactionGroupMessage = useCallback(
    async (
      factionId: string,
      senderId: string,
      body: string
    ): Promise<{ success: boolean; errorMessage?: string }> => {
      if (token && apiRanking) {
        try {
          const res = await invoke<{ message_id: string; error_message: string }>(
            "grpc_send_hackerboard_faction_message",
            { token, body: body.trim() }
          );
          if (res.error_message) return { success: false, errorMessage: res.error_message };
        } catch {
          return { success: false, errorMessage: "__network__" };
        }
        await loadFactionRoomMessages(factionId);
        return { success: true };
      }
      const message: FactionGroupMessage = {
        id: `fg-${Date.now()}`,
        factionId,
        senderId,
        body,
        timestamp: Date.now(),
      };
      setFactionGroupMessages((prev) => ({
        ...prev,
        [factionId]: [...(prev[factionId] ?? []), message],
      }));
      return { success: true };
    },
    [token, apiRanking, loadFactionRoomMessages]
  );

  const hasMoreOlderDmMessages = useCallback(
    (conversationId: string) => !!serverDmHasOlder[conversationId],
    [serverDmHasOlder]
  );
  const hasMoreOlderFactionMessages = useCallback(
    (factionId: string) => !!serverFactionHasOlder[factionId],
    [serverFactionHasOlder]
  );

  const value: HackerboardContextValue = useMemo(
    () => ({
      hackers,
      factions: factionsWithRank,
      feed,
      userLikedPostIds,
      feedLanguageFilter,
      setFeedLanguageFilter,
      composePostLanguage,
      setComposePostLanguage,
      feedLoading,
      feedLoadingMore,
      feedRefreshing,
      feedHasMore,
      feedError,
      clusterRankingActive: apiRanking !== null,
      rankingError,
      retryRanking,
      refreshFeed,
      loadMoreFeed,
      searchHackers: (query: string) => {
        const q = query.trim().toLowerCase();
        if (!q) return hackers;
        return hackers.filter((h) => h.username.toLowerCase().includes(q));
      },
      searchFactions: (query: string) => {
        const q = query.trim().toLowerCase();
        if (!q) return factionsWithRank;
        return factionsWithRank.filter((f) => f.name.toLowerCase().includes(q));
      },
      addFeedPost,
      toggleLike,
      getDmConversations,
      getDmMessages,
      sendDm,
      refreshDmConversation,
      refreshHackerboardMessaging,
      hasMoreOlderDmMessages,
      loadOlderDmMessages,
      hasMoreOlderFactionMessages,
      loadOlderFactionMessages,
      getFactionGroupMessages,
      sendFactionGroupMessage,
      getEffectiveFactionId,
      createFaction,
      leaveFaction,
      factionInvitesIncoming,
      sendFactionInvite,
      acceptFactionInvite,
      declineFactionInvite,
      acceptServerFactionInvite,
      declineServerFactionInvite,
      blockedPlayerIds,
      isBlockedByMe,
      blockPlayer,
      unblockPlayer,
      canSendFactionInvite,
      sendFactionInviteByUsername,
      factionInvitesOutgoing,
      refreshFactionInvitesOutgoing: loadFactionInvitesOutgoing,
      cancelFactionInviteOutgoing,
    }),
    [
      hackers,
      factionsWithRank,
      feed,
      userLikedPostIds,
      feedLanguageFilter,
      composePostLanguage,
      feedLoading,
      feedLoadingMore,
      feedRefreshing,
      feedHasMore,
      feedError,
      apiRanking,
      rankingError,
      retryRanking,
      refreshFeed,
      loadMoreFeed,
      addFeedPost,
      setFeedLanguageFilter,
      setComposePostLanguage,
      toggleLike,
      getDmConversations,
      getDmMessages,
      sendDm,
      refreshDmConversation,
      refreshHackerboardMessaging,
      hasMoreOlderDmMessages,
      loadOlderDmMessages,
      hasMoreOlderFactionMessages,
      loadOlderFactionMessages,
      getFactionGroupMessages,
      sendFactionGroupMessage,
      getEffectiveFactionId,
      createFaction,
      leaveFaction,
      factionInvitesIncoming,
      sendFactionInvite,
      acceptFactionInvite,
      declineFactionInvite,
      acceptServerFactionInvite,
      declineServerFactionInvite,
      blockedPlayerIds,
      isBlockedByMe,
      blockPlayer,
      unblockPlayer,
      canSendFactionInvite,
      sendFactionInviteByUsername,
      factionInvitesOutgoing,
      loadFactionInvitesOutgoing,
      cancelFactionInviteOutgoing,
    ]
  );

  return (
    <HackerboardContext.Provider value={value}>
      {children}
    </HackerboardContext.Provider>
  );
}

export function useHackerboard(): HackerboardContextValue {
  const ctx = useContext(HackerboardContext);
  if (!ctx) throw new Error("useHackerboard must be used within HackerboardProvider");
  return ctx;
}

export function useHackerboardOptional(): HackerboardContextValue | null {
  return useContext(HackerboardContext);
}
