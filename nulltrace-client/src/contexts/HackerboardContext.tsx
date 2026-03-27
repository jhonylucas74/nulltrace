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
const MOCK_FACTIONS: Faction[] = [
  { id: "f1", name: "Zero Day Collective", memberIds: ["h1", "h2", "h6", "h11"] },
  { id: "f2", name: "Null Protocol", memberIds: ["h3", "h5", "h10", "h14"] },
  { id: "f3", name: "Deep Signal", memberIds: ["h7", "h9", "h13"] },
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

/** Canonical conversation id for a pair of hackers. */
export function getConversationId(userId1: string, userId2: string): string {
  return `dm-${[userId1, userId2].sort().join("-")}`;
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
  feedError: string | null;
  refreshFeed: () => void;
  searchHackers: (query: string) => HackerWithRank[];
  searchFactions: (query: string) => FactionWithRank[];
  addFeedPost: (post: Omit<FeedPost, "id" | "timestamp"> & { language: FeedPostLanguage }) => Promise<void>;
  toggleLike: (postId: string) => Promise<void>;
  getDmConversations: (currentUserId: string) => DmConversationItem[];
  getDmMessages: (conversationId: string) => DMMessage[];
  sendDm: (senderId: string, otherParticipantId: string, body: string) => void;
  getFactionGroupMessages: (factionId: string) => FactionGroupMessage[];
  sendFactionGroupMessage: (factionId: string, senderId: string, body: string) => void;
  getEffectiveFactionId: (userId: string) => string | null;
  createFaction: (name: string, creatorUserId: string) => Promise<Faction | null>;
  leaveFaction: (userId: string) => void | Promise<void>;
  sendFactionInvite: (fromUserId: string, toUserId: string, factionId: string) => void;
  acceptFactionInvite: (messageId: string, acceptingUserId: string) => void;
  declineFactionInvite: (messageId: string) => void;
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
  const { token } = useAuth();
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
  const [feedError, setFeedError] = useState<string | null>(null);
  const [userLikedPostIds, setUserLikedPostIds] = useState<Set<string>>(new Set());
  const [dmConversations, setDmConversations] = useState<Record<string, DMMessage[]>>(buildInitialDmConversations);
  const [factionGroupMessages, setFactionGroupMessages] = useState<Record<string, FactionGroupMessage[]>>(
    buildInitialFactionGroupMessages
  );
  const [factions, setFactions] = useState<Faction[]>(() => [...MOCK_FACTIONS]);
  const [membershipOverlay, setMembershipOverlay] = useState<Record<string, string | null>>({});

  const [apiRanking, setApiRanking] = useState<{ hackers: Hacker[]; factions: Faction[] } | null>(null);
  const [rankingRefreshTrigger, setRankingRefreshTrigger] = useState(0);

  const fetchRanking = useCallback(async () => {
    if (!token) {
      setApiRanking(null);
      return;
    }
    try {
      const res = await invoke<{
        entries: Array<{
          rank: number;
          player_id: string;
          username: string;
          points: number;
          faction_id: string;
          faction_name: string;
        }>;
        error_message: string;
      }>("grpc_get_ranking", { token });
      if (res.error_message) {
        setApiRanking(null);
        return;
      }
      const hackersFromApi: Hacker[] = res.entries.map((e) => ({
        id: e.player_id,
        username: e.username,
        points: e.points,
        factionId: e.faction_id || null,
      }));
      const factionMap = new Map<string, { name: string; memberIds: string[] }>();
      for (const e of res.entries) {
        if (!e.faction_id) continue;
        const cur = factionMap.get(e.faction_id);
        if (cur) {
          cur.memberIds.push(e.player_id);
        } else {
          factionMap.set(e.faction_id, { name: e.faction_name || "Faction", memberIds: [e.player_id] });
        }
      }
      const factionsFromApi: Faction[] = Array.from(factionMap.entries()).map(([id, v]) => ({
        id,
        name: v.name,
        memberIds: v.memberIds,
      }));
      setApiRanking({ hackers: hackersFromApi, factions: factionsFromApi });
    } catch {
      setApiRanking(null);
    }
  }, [token]);

  useEffect(() => {
    fetchRanking();
  }, [fetchRanking, rankingRefreshTrigger]);

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

  const fetchFeed = useCallback(async () => {
    if (!token) {
      setFeed([]);
      setUserLikedPostIds(new Set());
      setFeedError(null);
      setFeedLoading(false);
      return;
    }
    setFeedLoading(true);
    setFeedError(null);
    try {
      const language_filter = feedLanguageFilter === "all" ? "" : feedLanguageFilter;
      const res = await invoke<{
        posts: Array<{
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
        }>;
        error_message: string;
      }>("grpc_list_feed_posts", {
        token,
        languageFilter: language_filter,
        limit: 100,
      });
      if (res.error_message) {
        setFeed([]);
        setUserLikedPostIds(new Set());
        setFeedError(res.error_message);
        return;
      }
      const mapped = res.posts.map((p) => mapApiEntryToFeedPost(p));
      setFeed(mapped);
      const liked = new Set<string>();
      for (const p of res.posts) {
        if (p.liked_by_me) liked.add(p.id);
      }
      setUserLikedPostIds(liked);
    } catch {
      setFeed([]);
      setUserLikedPostIds(new Set());
      setFeedError("Failed to load feed");
    } finally {
      setFeedLoading(false);
    }
  }, [token, feedLanguageFilter]);

  useEffect(() => {
    void fetchFeed();
  }, [fetchFeed]);

  const refreshFeed = useCallback(() => {
    void fetchFeed();
  }, [fetchFeed]);

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
          return { id: res.faction_id, name: res.name, memberIds: [creatorUserId] };
        } catch {
          return null;
        }
      }
      const id = `f-${Date.now()}`;
      const faction: Faction = { id, name, memberIds: [creatorUserId] };
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
    (fromUserId: string, toUserId: string, factionId: string) => {
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
    },
    [factions]
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
      const res = await invoke<{ post: unknown; error_message: string }>("grpc_create_feed_post", {
        token,
        body: post.body,
        language: post.language,
        replyToPostId: post.replyToId ?? "",
      });
      if (res.error_message) {
        throw new Error(res.error_message);
      }
      await fetchFeed();
    },
    [token, fetchFeed]
  );

  const toggleLike = useCallback(
    async (postId: string) => {
      if (!token) return;
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
    },
    [token]
  );

  const getDmConversations = useCallback(
    (currentUserId: string): DmConversationItem[] => {
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
      return items.sort((a, b) => {
        const msgsA = dmConversations[a.conversationId] ?? [];
        const msgsB = dmConversations[b.conversationId] ?? [];
        const lastA = msgsA[msgsA.length - 1]?.timestamp ?? 0;
        const lastB = msgsB[msgsB.length - 1]?.timestamp ?? 0;
        return lastB - lastA;
      });
    },
    [dmConversations]
  );

  const getDmMessages = useCallback(
    (conversationId: string): DMMessage[] => {
      const msgs = dmConversations[conversationId] ?? [];
      return [...msgs].sort((a, b) => a.timestamp - b.timestamp);
    },
    [dmConversations]
  );

  const sendDm = useCallback((senderId: string, otherParticipantId: string, body: string) => {
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
  }, []);

  const getFactionGroupMessages = useCallback(
    (factionId: string): FactionGroupMessage[] => {
      const msgs = factionGroupMessages[factionId] ?? [];
      return [...msgs].sort((a, b) => a.timestamp - b.timestamp);
    },
    [factionGroupMessages]
  );

  const sendFactionGroupMessage = useCallback((factionId: string, senderId: string, body: string) => {
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
  }, []);

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
      feedError,
      refreshFeed,
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
      getFactionGroupMessages,
      sendFactionGroupMessage,
      getEffectiveFactionId,
      createFaction,
      leaveFaction,
      sendFactionInvite,
      acceptFactionInvite,
      declineFactionInvite,
    }),
    [
      hackers,
      factionsWithRank,
      feed,
      userLikedPostIds,
      feedLanguageFilter,
      composePostLanguage,
      feedLoading,
      feedError,
      refreshFeed,
      addFeedPost,
      setFeedLanguageFilter,
      setComposePostLanguage,
      toggleLike,
      getDmConversations,
      getDmMessages,
      sendDm,
      getFactionGroupMessages,
      sendFactionGroupMessage,
      getEffectiveFactionId,
      createFaction,
      leaveFaction,
      sendFactionInvite,
      acceptFactionInvite,
      declineFactionInvite,
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
