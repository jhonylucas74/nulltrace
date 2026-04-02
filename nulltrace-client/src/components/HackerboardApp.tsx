import { useState, useMemo, useRef, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
  MessageSquare,
  Trophy,
  Search,
  Flag,
  CheckCircle,
  Info,
  User,
  MessageCircle,
  Heart,
  Mail,
  Users,
  ArrowLeft,
  Loader2,
  Globe,
  RotateCw,
  ShieldAlert,
  UserX,
  UserCircle,
} from "lucide-react";
import { useAuth } from "../contexts/AuthContext";
import { useFilePicker, getDefaultInitialPath } from "../contexts/FilePickerContext";
import { pixelArtDataUrlFromNtpixelsBase64 } from "../lib/pixelArt";
import {
  useHackerboard,
  type FeedPost,
  type FeedPostLanguage,
  type FactionWithRank,
  getConversationId,
} from "../contexts/HackerboardContext";
import Modal from "./Modal";
import styles from "./HackerboardApp.module.css";

type Section = "feed" | "rankings" | "messages" | "group" | "profile";
type RankTab = "hackers" | "factions";

/** Synthetic "other participant" id for the server-backed faction invites pane. */
const SERVER_FACTION_INVITES_PANEL_ID = "__server_faction_invites__";

/** Matches non-cluster server stub (`nulltrace-core/src/server/main.rs`). */
const FEED_CLUSTER_STUB_ERROR = "Use the unified cluster binary for Hackerboard feed";

const RANKING_CLUSTER_STUB_ERROR = "Use the unified cluster binary for ranking";

const REPORT_REASON_VALUES = ["spam", "harassment", "hate_or_abuse", "other"] as const;

function reportedPostIdsStorageKey(playerId: string): string {
  return `nulltrace.hackerboard.reportedPostIds.${playerId}`;
}

function resolveFeedErrorDisplay(
  feedError: string | null,
  t: (key: string) => string
): { message: string; tone: "error" | "info" } | null {
  if (!feedError) return null;
  if (feedError === FEED_CLUSTER_STUB_ERROR) {
    return { message: t("feedClusterRequired"), tone: "info" };
  }
  if (feedError === "Failed to load feed") {
    return { message: t("feedError"), tone: "error" };
  }
  return { message: feedError, tone: "error" };
}

function resolveRankingErrorDisplay(
  rankingError: string | null,
  t: (key: string) => string
): { message: string; tone: "error" | "info" } | null {
  if (!rankingError) return null;
  if (rankingError === RANKING_CLUSTER_STUB_ERROR) {
    return { message: t("rankingClusterRequired"), tone: "info" };
  }
  if (rankingError === "__ranking_network__") {
    return { message: t("rankingNetworkError"), tone: "error" };
  }
  return { message: rankingError, tone: "error" };
}

/** Map server / client error tokens to `hackerboard` i18n keys. */
function mapHackerboardMessagingError(raw: string | undefined, t: (key: string) => string): string {
  if (!raw) return t("messagingSendFailed");
  if (raw === "__peer_unavailable__") return t("dmErrorPeerUnavailable");
  if (raw === "__network__") return t("messagingNetworkError");
  if (raw === "__dm_blocked__") return t("dmErrorBlocked");
  const keys: Record<string, string> = {
    "Player not found": "dmErrorPlayerNotFound",
    "Cannot message yourself": "dmErrorSelf",
    "Message is empty": "messagingEmptyBody",
    "You are not in a faction": "factionChatErrorNotInFaction",
  };
  const key = keys[raw];
  if (key) return t(key);
  if (raw.startsWith("Message exceeds")) return t("messagingBodyTooLong");
  return t("messagingSendFailed");
}

function mapFactionInviteUiError(raw: string | undefined, t: (key: string) => string): string {
  if (!raw) return t("inviteSendFailed");
  if (raw === "__invite_username_empty__") return t("inviteUsernameRequired");
  if (raw === "__network__") return t("messagingNetworkError");
  const keys: Record<string, string> = {
    "Player not found": "dmErrorPlayerNotFound",
    "You are not in a faction": "factionChatErrorNotInFaction",
    "Username is required": "inviteUsernameRequired",
    "Cannot invite yourself": "inviteErrorSelf",
    "Only the faction creator can send invites": "inviteErrorCreatorOnly",
    "Player is already in a faction": "inviteErrorTargetInFaction",
    "An invite to this player for this faction is already pending": "inviteErrorAlreadyPending",
    "Inviter not found": "inviteSendFailed",
    "You are not a member of that faction": "inviteSendFailed",
    "Faction not found": "inviteSendFailed",
    "Not allowed to cancel this invite": "cancelInviteNotAllowed",
    "Invite is not pending": "cancelInviteNotPending",
    "Invite not found": "cancelInviteNotFound",
    "Invalid invite id": "cancelInviteInvalidId",
    "That player is banned from this faction": "inviteErrorBannedFromFaction",
  };
  const key = keys[raw];
  if (key) return t(key);
  return raw;
}

function mapKickFactionMemberError(raw: string | undefined, t: (key: string) => string): string {
  if (!raw) return t("kickFactionFailed");
  if (raw === "__invite_username_empty__") return t("inviteUsernameRequired");
  if (raw === "__network__") return t("messagingNetworkError");
  if (raw === "Not available offline") return t("kickFactionOffline");
  const keys: Record<string, string> = {
    "Username is required": "inviteUsernameRequired",
    "Player not found": "dmErrorPlayerNotFound",
    "You are not in a faction": "factionChatErrorNotInFaction",
    "Faction has no creator": "kickFactionFailed",
    "Only the faction creator can kick members": "kickFactionCreatorOnly",
    "Cannot kick the faction leader": "kickFactionCannotKickLeader",
    "That player is not in your faction": "kickFactionTargetNotMember",
  };
  const key = keys[raw];
  if (key) return t(key);
  return raw;
}

function mapUnbanFactionMemberError(raw: string | undefined, t: (key: string) => string): string {
  if (!raw) return t("unbanFactionFailed");
  if (raw === "__invite_username_empty__") return t("inviteUsernameRequired");
  if (raw === "__network__") return t("messagingNetworkError");
  if (raw === "Not available offline") return t("kickFactionOffline");
  const keys: Record<string, string> = {
    "Username is required": "inviteUsernameRequired",
    "Player not found": "dmErrorPlayerNotFound",
    "You are not in a faction": "factionChatErrorNotInFaction",
    "Faction has no creator": "unbanFactionFailed",
    "Only the faction creator can unban players": "unbanFactionCreatorOnly",
    "That player is not banned from this faction": "unbanFactionNotBanned",
  };
  const key = keys[raw];
  if (key) return t(key);
  return raw;
}

function mapBlockActionError(raw: string | undefined, t: (key: string) => string): string {
  if (!raw) return t("blockActionFailed");
  if (raw === "__network__") return t("messagingNetworkError");
  const keys: Record<string, string> = {
    "Username is required": "inviteUsernameRequired",
    "Player not found": "dmErrorPlayerNotFound",
    "Cannot block yourself": "blockErrorSelf",
    "Not blocked": "unblockErrorNotBlocked",
    "Not signed in": "signInMessages",
  };
  const key = keys[raw];
  if (key) return t(key);
  return raw;
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  const now = Date.now();
  const diff = now - ts;
  if (diff < 60_000) return "Just now";
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86400_000) return `${Math.floor(diff / 3600_000)}h ago`;
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

function getAuthorHandle(
  post: FeedPost,
  hackers: { id: string; username: string }[]
): string {
  if (post.authorId) {
    const h = hackers.find((x) => x.id === post.authorId);
    return h?.username ?? "unknown";
  }
  if (post.type === "system") return "Hackerboard";
  if (post.type === "hacked" || post.type === "mission") return "Hackerboard";
  return "unknown";
}

function CreateFactionForm({ onCreate }: { onCreate: (name: string) => void }) {
  const { t } = useTranslation("hackerboard");
  const [name, setName] = useState("");
  return (
    <form
      className={styles.createFactionForm}
      onSubmit={(e) => {
        e.preventDefault();
        const n = name.trim();
        if (n) {
          onCreate(n);
          setName("");
        }
      }}
    >
      <input
        type="text"
        className={styles.createFactionInput}
        placeholder={t("createFactionNamePlaceholder")}
        value={name}
        onChange={(e) => setName(e.target.value)}
        aria-label={t("createFactionNameAria")}
      />
      <button type="submit" className={styles.createFactionBtn} disabled={!name.trim()}>
        {t("createFactionSubmit")}
      </button>
    </form>
  );
}

type GroupTab = "chat" | "members";

function GroupWithFaction({
  currentUserHacker,
  currentUserFaction,
  hackers,
  currentUserFactionGroupMessages,
  groupMessageListRef,
  groupMessageText,
  setGroupMessageText,
  onSendGroupMessage,
  onLeaveFaction,
  inviteFeedback,
  groupSendError,
  showLoadOlderMessages,
  onLoadOlderMessages,
  loadOlderMessagesPending,
  loadOlderMessagesLabel,
  loadingOlderMessagesLabel,
  inviteUsername,
  setInviteUsername,
  onSubmitUsernameInvite,
  usernameInvitePending,
  outgoingInvites,
  onCancelOutgoingInvite,
  cancelOutgoingBusyId,
  clusterRankingActive,
  factionEmblemDataUrl,
  showFactionEmblemVmButton,
  onChooseFactionEmblemFromVm,
}: {
  currentUserHacker: { id: string };
  currentUserFaction: FactionWithRank;
  hackers: { id: string; username: string }[];
  currentUserFactionGroupMessages: { id: string; senderId: string; body: string; timestamp: number }[];
  groupMessageListRef: React.RefObject<HTMLDivElement | null>;
  groupMessageText: string;
  setGroupMessageText: (v: string) => void;
  onSendGroupMessage: (e: React.FormEvent) => void;
  onLeaveFaction: () => void;
  inviteFeedback?: string | null;
  groupSendError?: string | null;
  showLoadOlderMessages?: boolean;
  onLoadOlderMessages?: () => void;
  loadOlderMessagesPending?: boolean;
  loadOlderMessagesLabel?: string;
  loadingOlderMessagesLabel?: string;
  inviteUsername: string;
  setInviteUsername: (v: string) => void;
  onSubmitUsernameInvite: (e: React.FormEvent) => void;
  usernameInvitePending: boolean;
  outgoingInvites: Array<{
    inviteId: string;
    toUsername: string;
    fromUsername: string;
    createdAtMs: number;
  }>;
  onCancelOutgoingInvite: (inviteId: string) => void | Promise<void>;
  cancelOutgoingBusyId: string | null;
  clusterRankingActive: boolean;
  factionEmblemDataUrl: string | null;
  showFactionEmblemVmButton: boolean;
  onChooseFactionEmblemFromVm?: () => void;
}) {
  const { t } = useTranslation("hackerboard");
  const {
    factionBannedMembers,
    refreshFactionBannedMembers,
    kickFactionMember,
    unbanFactionMember,
  } = useHackerboard();
  const [groupTab, setGroupTab] = useState<GroupTab>("chat");
  const [leaveConfirm, setLeaveConfirm] = useState(false);
  const [kickConfirm, setKickConfirm] = useState<{ username: string; banFromRejoin: boolean } | null>(
    null
  );
  const [kickBusy, setKickBusy] = useState(false);
  const [unbanBusyUsername, setUnbanBusyUsername] = useState<string | null>(null);
  const [factionMemberActionError, setFactionMemberActionError] = useState<string | null>(null);

  const isFactionCreator =
    !!currentUserFaction.creatorId && currentUserFaction.creatorId === currentUserHacker.id;

  useEffect(() => {
    if (groupTab === "members" && clusterRankingActive && isFactionCreator) {
      void refreshFactionBannedMembers();
    }
  }, [groupTab, clusterRankingActive, isFactionCreator, refreshFactionBannedMembers]);

  return (
    <>
      <div className={styles.factionHeaderRow}>
        {factionEmblemDataUrl ? (
          <img
            src={factionEmblemDataUrl}
            alt=""
            width={32}
            height={32}
            className={styles.factionEmblemImg}
          />
        ) : null}
        <div className={styles.groupTitle}>{currentUserFaction.name}</div>
        {showFactionEmblemVmButton && onChooseFactionEmblemFromVm ? (
          <button type="button" className={styles.vmPixelFileBtn} onClick={onChooseFactionEmblemFromVm}>
            {t("setFactionEmblemFromVmFile")}
          </button>
        ) : null}
      </div>
      <p className={styles.factionSummary} aria-live="polite">
        {t("factionSummaryStats", {
          rank: currentUserFaction.rank,
          count: currentUserFaction.memberIds.length,
          points: currentUserFaction.totalPoints.toLocaleString(),
        })}
      </p>
      <div className={styles.groupTabs}>
        <button
          type="button"
          className={groupTab === "chat" ? styles.groupTabActive : styles.groupTab}
          onClick={() => setGroupTab("chat")}
        >
          {t("factionTabChat")}
        </button>
        <button
          type="button"
          className={groupTab === "members" ? styles.groupTabActive : styles.groupTab}
          onClick={() => setGroupTab("members")}
        >
          {t("factionTabMembers")}
        </button>
      </div>
      {groupTab === "chat" && (
        <>
          {groupSendError ? <p className={styles.inviteFeedbackError}>{groupSendError}</p> : null}
          {showLoadOlderMessages && onLoadOlderMessages ? (
            <div className={styles.loadOlderRow}>
              <button
                type="button"
                className={styles.loadOlderBtn}
                onClick={onLoadOlderMessages}
                disabled={loadOlderMessagesPending}
              >
                {loadOlderMessagesPending
                  ? (loadingOlderMessagesLabel ?? "…")
                  : (loadOlderMessagesLabel ?? "Load older")}
              </button>
            </div>
          ) : null}
          <div className={styles.groupMessageList} ref={groupMessageListRef as React.LegacyRef<HTMLDivElement>}>
            {currentUserFactionGroupMessages.map((msg) => {
              const senderName = hackers.find((h) => h.id === msg.senderId)?.username ?? msg.senderId;
              const isFromSelf = msg.senderId === currentUserHacker.id;
              return (
                <div
                  key={msg.id}
                  className={`${styles.messageBubble} ${isFromSelf ? styles.messageBubbleSelf : ""}`}
                >
                  {!isFromSelf && <span className={styles.messageSender}>{senderName}</span>}
                  <p className={styles.messageBody}>{msg.body}</p>
                  <span className={styles.messageTime}>{formatTime(msg.timestamp)}</span>
                </div>
              );
            })}
          </div>
          <form className={styles.messageInputWrap} onSubmit={onSendGroupMessage}>
            <input
              type="text"
              className={styles.messageInput}
              placeholder={t("groupChatPlaceholder")}
              value={groupMessageText}
              onChange={(e) => setGroupMessageText(e.target.value)}
              aria-label={t("groupChatAria")}
            />
            <button type="submit" className={styles.messageSendBtn} disabled={!groupMessageText.trim()}>
              {t("factionChatSend")}
            </button>
          </form>
        </>
      )}
      {groupTab === "members" && (
        <div className={styles.membersArea}>
          <h3 className={styles.membersSectionTitle}>{t("inviteByUsernameTitle")}</h3>
          {inviteFeedback ? <p className={styles.inviteFeedbackError}>{inviteFeedback}</p> : null}
          <form className={styles.usernameInviteForm} onSubmit={onSubmitUsernameInvite}>
            <input
              type="text"
              className={styles.usernameInviteInput}
              placeholder={t("inviteUsernamePlaceholder")}
              value={inviteUsername}
              onChange={(e) => setInviteUsername(e.target.value)}
              autoComplete="off"
              aria-label={t("inviteUsernameAria")}
              disabled={usernameInvitePending}
            />
            <button type="submit" className={styles.inviteMemberBtn} disabled={usernameInvitePending || !inviteUsername.trim()}>
              {t("sendInvite")}
            </button>
          </form>
          {clusterRankingActive ? (
            <>
              <h3 className={styles.membersSectionTitle}>{t("outgoingInvitesTitle")}</h3>
              {outgoingInvites.length === 0 ? (
                <p className={styles.emptyState}>{t("outgoingInvitesEmpty")}</p>
              ) : (
                <ul className={styles.outgoingInviteList}>
                  {outgoingInvites.map((inv) => (
                    <li key={inv.inviteId} className={styles.outgoingInviteRow}>
                      <span className={styles.outgoingInviteMeta}>
                        {t("outgoingInviteRow", { user: inv.toUsername, from: inv.fromUsername })}
                      </span>
                      <button
                        type="button"
                        className={styles.outgoingInviteCancelBtn}
                        onClick={() => void onCancelOutgoingInvite(inv.inviteId)}
                        disabled={cancelOutgoingBusyId === inv.inviteId}
                      >
                        {t("cancelInvite")}
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </>
          ) : null}
          <h3 className={styles.membersSectionTitle}>
            {t("membersTitle")}{" "}
            {currentUserFaction.memberIds.length > 0 ? `(${currentUserFaction.memberIds.length})` : ""}
          </h3>
          {factionMemberActionError ? (
            <p className={styles.inviteFeedbackError} role="alert">
              {factionMemberActionError}
            </p>
          ) : null}
          <ul className={styles.memberList}>
            {currentUserFaction.memberIds.map((id) => {
              const h = hackers.find((x) => x.id === id);
              const uname = h?.username ?? "";
              const showKick =
                clusterRankingActive && isFactionCreator && id !== currentUserHacker.id && !!uname;
              const isConfirming = kickConfirm?.username === uname;
              return (
                <li key={id} className={styles.memberRow}>
                  <span className={styles.memberItemName}>{uname || id}</span>
                  {showKick ? (
                    <div className={styles.memberKickRow}>
                      {isConfirming ? (
                        <>
                          <span className={styles.kickConfirmLabel}>
                            {kickConfirm?.banFromRejoin
                              ? t("factionKickConfirmBan")
                              : t("factionKickConfirm")}
                          </span>
                          <button
                            type="button"
                            className={styles.kickConfirmBtn}
                            disabled={kickBusy}
                            onClick={() => {
                              if (!kickConfirm) return;
                              setKickBusy(true);
                              setFactionMemberActionError(null);
                              void (async () => {
                                const r = await kickFactionMember(kickConfirm.username, {
                                  banFromRejoin: kickConfirm.banFromRejoin,
                                });
                                setKickBusy(false);
                                if (!r.success) {
                                  setFactionMemberActionError(
                                    mapKickFactionMemberError(r.errorMessage, t)
                                  );
                                  return;
                                }
                                setKickConfirm(null);
                              })();
                            }}
                          >
                            {t("factionKickConfirmYes")}
                          </button>
                          <button
                            type="button"
                            className={styles.kickCancelBtn}
                            disabled={kickBusy}
                            onClick={() => setKickConfirm(null)}
                          >
                            {t("leaveFactionCancel")}
                          </button>
                        </>
                      ) : (
                        <>
                          <button
                            type="button"
                            className={styles.kickMemberBtn}
                            onClick={() => {
                              setFactionMemberActionError(null);
                              setKickConfirm({ username: uname, banFromRejoin: false });
                            }}
                          >
                            {t("factionKickMember")}
                          </button>
                          <button
                            type="button"
                            className={styles.kickBanMemberBtn}
                            onClick={() => {
                              setFactionMemberActionError(null);
                              setKickConfirm({ username: uname, banFromRejoin: true });
                            }}
                          >
                            {t("factionKickAndBanMember")}
                          </button>
                        </>
                      )}
                    </div>
                  ) : null}
                </li>
              );
            })}
          </ul>
          {clusterRankingActive && isFactionCreator && factionBannedMembers.length > 0 ? (
            <>
              <h3 className={styles.membersSectionTitle}>{t("factionBannedSectionTitle")}</h3>
              <ul className={styles.factionBannedList}>
                {factionBannedMembers.map((b) => (
                  <li key={b.playerId} className={styles.factionBannedRow}>
                    <span className={styles.factionBannedName}>{b.username}</span>
                    <button
                      type="button"
                      className={styles.factionUnbanBtn}
                      disabled={unbanBusyUsername === b.username}
                      onClick={() => {
                        setFactionMemberActionError(null);
                        setUnbanBusyUsername(b.username);
                        void (async () => {
                          const r = await unbanFactionMember(b.username);
                          setUnbanBusyUsername(null);
                          if (!r.success) {
                            setFactionMemberActionError(
                              mapUnbanFactionMemberError(r.errorMessage, t)
                            );
                          }
                        })();
                      }}
                    >
                      {t("factionUnbanMember")}
                    </button>
                  </li>
                ))}
              </ul>
            </>
          ) : null}
          <div className={styles.leaveFactionWrap}>
            {leaveConfirm ? (
              <>
                <span className={styles.leaveConfirmText}>{t("leaveFactionConfirm")}</span>
                <button type="button" className={styles.leaveConfirmBtn} onClick={() => { onLeaveFaction(); setLeaveConfirm(false); }}>
                  {t("leaveFactionYes")}
                </button>
                <button type="button" className={styles.leaveCancelBtn} onClick={() => setLeaveConfirm(false)}>
                  {t("leaveFactionCancel")}
                </button>
              </>
            ) : (
              <button type="button" className={styles.leaveFactionBtn} onClick={() => setLeaveConfirm(true)}>
                {t("leaveFaction")}
              </button>
            )}
          </div>
        </div>
      )}
    </>
  );
}

function PostIcon({ type }: { type: FeedPost["type"] }) {
  const iconClass =
    type === "hacked"
      ? styles.postIconHacked
      : type === "mission"
        ? styles.postIconMission
        : type === "user"
          ? styles.postIconUser
          : styles.postIconSystem;
  return (
    <span className={`${styles.postIcon} ${iconClass}`}>
      {type === "hacked" && <Flag size={16} />}
      {type === "mission" && <CheckCircle size={16} />}
      {type === "user" && <User size={16} />}
      {type === "system" && <Info size={16} />}
    </span>
  );
}

export default function HackerboardApp() {
  const { t } = useTranslation("hackerboard");
  const { username, token, playerId } = useAuth();
  const {
    hackers,
    factions,
    feed,
    searchHackers,
    searchFactions,
    addFeedPost,
    toggleLike,
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
    refreshFeed,
    loadMoreFeed,
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
    acceptFactionInvite,
    declineFactionInvite,
    factionInvitesIncoming,
    acceptServerFactionInvite,
    declineServerFactionInvite,
    clusterRankingActive,
    rankingError,
    retryRanking,
    blockedPlayerIds,
    isBlockedByMe,
    blockPlayer,
    unblockPlayer,
    canSendFactionInvite,
    sendFactionInviteByUsername,
    factionInvitesOutgoing,
    cancelFactionInviteOutgoing,
    setHackerboardAvatarFromVmPath,
    setFactionEmblemFromVmPath,
  } = useHackerboard();
  const [section, setSection] = useState<Section>("feed");
  const [rankTab, setRankTab] = useState<RankTab>("hackers");
  const [selectedProfileUserId, setSelectedProfileUserId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [composeText, setComposeText] = useState("");
  const [feedLangMenuOpen, setFeedLangMenuOpen] = useState(false);
  const feedLangMenuRef = useRef<HTMLDivElement>(null);
  const [expandedThreadId, setExpandedThreadId] = useState<string | null>(null);
  const [threadReplyText, setThreadReplyText] = useState("");
  const [selectedDmConversationId, setSelectedDmConversationId] = useState<string | null>(null);
  const threadReplyInputRef = useRef<HTMLTextAreaElement>(null);
  const [selectedDmOtherParticipantId, setSelectedDmOtherParticipantId] = useState<string | null>(null);
  const [dmMessageText, setDmMessageText] = useState("");
  const [groupMessageText, setGroupMessageText] = useState("");
  const [groupInviteFeedback, setGroupInviteFeedback] = useState<string | null>(null);
  const [dmSendError, setDmSendError] = useState<string | null>(null);
  const [groupSendError, setGroupSendError] = useState<string | null>(null);
  const [loadingOlderDm, setLoadingOlderDm] = useState(false);
  const [loadingOlderFaction, setLoadingOlderFaction] = useState(false);
  const [serverInviteActionError, setServerInviteActionError] = useState<string | null>(null);
  const [groupInviteUsername, setGroupInviteUsername] = useState("");
  const [groupInviteBusy, setGroupInviteBusy] = useState(false);
  const [cancelOutgoingBusyId, setCancelOutgoingBusyId] = useState<string | null>(null);
  const [profileBlockBusy, setProfileBlockBusy] = useState(false);
  const [profileActionError, setProfileActionError] = useState<string | null>(null);
  const [profileInviteToast, setProfileInviteToast] = useState<string | null>(null);
  const profileInviteToastTimerRef = useRef<number | null>(null);
  const composeTextareaRef = useRef<HTMLTextAreaElement>(null);
  const dmMessageListRef = useRef<HTMLDivElement>(null);
  const suppressDmAutoScrollRef = useRef(false);
  const groupMessageListRef = useRef<HTMLDivElement>(null);
  const suppressGroupAutoScrollRef = useRef(false);
  const feedAreaRef = useRef<HTMLDivElement>(null);
  const feedLoadMoreSentinelRef = useRef<HTMLDivElement>(null);
  /** Clears heart animation classes after keyframes finish (per post). */
  const heartAnimTimersRef = useRef<Record<string, number>>({});
  const [heartAnimByPost, setHeartAnimByPost] = useState<Record<string, "like" | "unlike">>({});
  const [reportedPostIds, setReportedPostIds] = useState<Set<string>>(() => new Set());
  const [reportModalPost, setReportModalPost] = useState<FeedPost | null>(null);
  const [reportReason, setReportReason] = useState<string>("");

  useEffect(() => {
    if (!playerId) {
      setReportedPostIds(new Set());
      return;
    }
    try {
      const raw = localStorage.getItem(reportedPostIdsStorageKey(playerId));
      if (!raw) {
        setReportedPostIds(new Set());
        return;
      }
      const parsed = JSON.parse(raw) as unknown;
      if (!Array.isArray(parsed)) {
        setReportedPostIds(new Set());
        return;
      }
      setReportedPostIds(new Set(parsed.filter((x): x is string => typeof x === "string")));
    } catch {
      setReportedPostIds(new Set());
    }
  }, [playerId]);

  const triggerHeartAnim = useCallback((postId: string, kind: "like" | "unlike") => {
    const timers = heartAnimTimersRef.current;
    const prev = timers[postId];
    if (prev !== undefined) window.clearTimeout(prev);
    setHeartAnimByPost((s) => ({ ...s, [postId]: kind }));
    timers[postId] = window.setTimeout(() => {
      setHeartAnimByPost((s) => {
        const rest = { ...s };
        delete rest[postId];
        return rest;
      });
      delete timers[postId];
    }, 500);
  }, []);
  const isScrollingRef = useRef(false);
  const scrollIdleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isNearTopRef = useRef(true);

  const currentUserHacker = useMemo(
    () => (username ? hackers.find((h) => h.username === username) ?? null : null),
    [username, hackers]
  );

  const filteredHackers = useMemo(
    () => searchHackers(searchQuery),
    [searchQuery, searchHackers]
  );
  const filteredFactions = useMemo(
    () => searchFactions(searchQuery),
    [searchQuery, searchFactions]
  );

  const feedErrorDisplay = useMemo(
    () => resolveFeedErrorDisplay(feedError, t),
    [feedError, t]
  );

  const rankingErrorDisplay = useMemo(
    () => resolveRankingErrorDisplay(rankingError, t),
    [rankingError, t]
  );

  useEffect(() => {
    if (!feedLangMenuOpen) return;
    const onPointerDown = (e: PointerEvent) => {
      const el = feedLangMenuRef.current;
      if (el && !el.contains(e.target as Node)) setFeedLangMenuOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setFeedLangMenuOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [feedLangMenuOpen]);

  const onFeedAreaScroll = useCallback(() => {
    const el = feedAreaRef.current;
    if (el) {
      isNearTopRef.current = el.scrollTop <= 8;
    }
    isScrollingRef.current = true;
    if (scrollIdleTimerRef.current) clearTimeout(scrollIdleTimerRef.current);
    scrollIdleTimerRef.current = setTimeout(() => {
      isScrollingRef.current = false;
      scrollIdleTimerRef.current = null;
    }, 300);
  }, []);

  useEffect(() => {
    return () => {
      if (scrollIdleTimerRef.current) clearTimeout(scrollIdleTimerRef.current);
    };
  }, []);

  useEffect(() => {
    const timers = heartAnimTimersRef.current;
    return () => {
      Object.values(timers).forEach((id) => window.clearTimeout(id));
    };
  }, []);

  useEffect(() => {
    const id = window.setInterval(() => {
      if (!token || section !== "feed") return;
      if (!isNearTopRef.current || isScrollingRef.current || feedLoading || feedLoadingMore || feedRefreshing) return;
      refreshFeed();
    }, 60_000);
    return () => clearInterval(id);
  }, [token, section, feedLoading, feedLoadingMore, feedRefreshing, refreshFeed]);

  const visibleFeed = useMemo(
    () =>
      feed.filter((p) => {
        if (reportedPostIds.has(p.id)) return false;
        if (p.authorId && blockedPlayerIds.has(p.authorId)) return false;
        return true;
      }),
    [feed, reportedPostIds, blockedPlayerIds]
  );

  /** Only root posts (no replyToId) appear in the main feed. */
  const rootPosts = useMemo(() => visibleFeed.filter((p) => !p.replyToId), [visibleFeed]);

  /** Replies per root post id, sorted by time. */
  const repliesByRootId = useMemo(() => {
    const map = new Map<string, FeedPost[]>();
    visibleFeed.forEach((p) => {
      if (p.replyToId) {
        const list = map.get(p.replyToId) ?? [];
        list.push(p);
        map.set(p.replyToId, list);
      }
    });
    map.forEach((list) => list.sort((a, b) => a.timestamp - b.timestamp));
    return map;
  }, [visibleFeed]);

  useEffect(() => {
    if (section !== "feed" || !feedHasMore || feedLoading || !token) return;
    const root = feedAreaRef.current;
    const target = feedLoadMoreSentinelRef.current;
    if (!root || !target) return;
    const obs = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) void loadMoreFeed();
      },
      { root, rootMargin: "120px", threshold: 0 }
    );
    obs.observe(target);
    return () => obs.disconnect();
  }, [section, feedHasMore, feedLoading, token, loadMoreFeed, rootPosts.length]);

  async function handlePostSubmit(e: React.FormEvent) {
    e.preventDefault();
    const body = composeText.trim();
    if (!body || !currentUserHacker || !token) return;
    try {
      await addFeedPost({
        type: "user",
        body,
        authorId: currentUserHacker.id,
        language: composePostLanguage,
      });
      setComposeText("");
    } catch {
      /* error surfaced via feedError on next refresh */
    }
  }

  function toggleThread(postId: string) {
    setExpandedThreadId((prev) => {
      const next = prev === postId ? null : postId;
      if (next) {
        setTimeout(() => threadReplyInputRef.current?.focus(), 100);
      }
      return next;
    });
    setThreadReplyText("");
  }

  async function handleThreadReplySubmit(e: React.FormEvent, rootPostId: string) {
    e.preventDefault();
    const body = threadReplyText.trim();
    if (!body || !currentUserHacker || !token) return;
    try {
      await addFeedPost({
        type: "user",
        body,
        authorId: currentUserHacker.id,
        replyToId: rootPostId,
        language: composePostLanguage,
      });
      setThreadReplyText("");
    } catch {
      /* ignore */
    }
  }

  const dmConversationsList = useMemo(
    () => (currentUserHacker ? getDmConversations(currentUserHacker.id) : []),
    [currentUserHacker, getDmConversations]
  );
  const selectedDmMessages = useMemo(
    () => (selectedDmConversationId ? getDmMessages(selectedDmConversationId) : []),
    [selectedDmConversationId, getDmMessages]
  );
  const effectiveFactionId = useMemo(
    () => (currentUserHacker ? getEffectiveFactionId(currentUserHacker.id) : null),
    [currentUserHacker, getEffectiveFactionId]
  );
  const currentUserFactionGroupMessages = useMemo(
    () => (effectiveFactionId ? getFactionGroupMessages(effectiveFactionId) : []),
    [effectiveFactionId, getFactionGroupMessages]
  );
  const currentUserFaction = useMemo(
    () => (effectiveFactionId ? factions.find((f) => f.id === effectiveFactionId) : null),
    [effectiveFactionId, factions]
  );
  const factionEmblemDataUrl = useMemo(() => {
    if (!currentUserFaction?.emblemPixelB64) return null;
    return pixelArtDataUrlFromNtpixelsBase64(currentUserFaction.emblemPixelB64);
  }, [currentUserFaction?.emblemPixelB64]);
  const profileUser = useMemo(
    () => (selectedProfileUserId ? hackers.find((h) => h.id === selectedProfileUserId) ?? null : null),
    [selectedProfileUserId, hackers]
  );
  const profilePosts = useMemo(
    () =>
      selectedProfileUserId
        ? visibleFeed.filter((p) => p.authorId === selectedProfileUserId && !p.replyToId && p.type === "user")
        : [],
    [selectedProfileUserId, visibleFeed]
  );

  function openReportModal(post: FeedPost) {
    setReportReason("");
    setReportModalPost(post);
  }

  function closeReportModal() {
    setReportModalPost(null);
    setReportReason("");
  }

  function submitReport() {
    if (!reportModalPost || !reportReason || !playerId) return;
    setReportedPostIds((prev) => {
      const next = new Set(prev);
      next.add(reportModalPost.id);
      try {
        localStorage.setItem(reportedPostIdsStorageKey(playerId), JSON.stringify([...next]));
      } catch {
        /* ignore quota / private mode */
      }
      return next;
    });
    if (expandedThreadId === reportModalPost.id) {
      setExpandedThreadId(null);
    }
    closeReportModal();
  }

  useEffect(() => {
    if (suppressDmAutoScrollRef.current) {
      suppressDmAutoScrollRef.current = false;
      return;
    }
    dmMessageListRef.current?.scrollTo({ top: dmMessageListRef.current.scrollHeight, behavior: "smooth" });
  }, [selectedDmMessages.length, factionInvitesIncoming.length, selectedDmOtherParticipantId]);
  useEffect(() => {
    if (section !== "group" || !currentUserFactionGroupMessages.length) return;
    if (suppressGroupAutoScrollRef.current) {
      suppressGroupAutoScrollRef.current = false;
      return;
    }
    groupMessageListRef.current?.scrollTo({ top: groupMessageListRef.current.scrollHeight, behavior: "smooth" });
  }, [section, currentUserFactionGroupMessages.length]);

  useEffect(() => {
    if (!token || !clusterRankingActive) return;
    if (section !== "messages" && section !== "group") return;
    void refreshHackerboardMessaging();
  }, [section, token, clusterRankingActive, refreshHackerboardMessaging]);

  useEffect(() => {
    if (section !== "messages") return;
    if (!token || !clusterRankingActive || !currentUserHacker) return;
    if (selectedDmOtherParticipantId === SERVER_FACTION_INVITES_PANEL_ID) return;
    if (!selectedDmOtherParticipantId) return;
    void refreshDmConversation(currentUserHacker.id, selectedDmOtherParticipantId);
  }, [
    section,
    token,
    clusterRankingActive,
    currentUserHacker,
    selectedDmOtherParticipantId,
    refreshDmConversation,
  ]);

  useEffect(() => {
    if (section !== "messages") setDmSendError(null);
  }, [section]);

  useEffect(() => {
    if (section !== "group") setGroupSendError(null);
  }, [section]);

  useEffect(() => {
    if (!token || !clusterRankingActive) return;
    if (section !== "messages" && section !== "group") return;
    const id = window.setInterval(() => {
      void refreshHackerboardMessaging();
    }, 60_000);
    return () => clearInterval(id);
  }, [token, clusterRankingActive, section, refreshHackerboardMessaging]);

  async function handleLoadOlderDm() {
    if (!currentUserHacker || !selectedDmOtherParticipantId) return;
    suppressDmAutoScrollRef.current = true;
    setLoadingOlderDm(true);
    try {
      await loadOlderDmMessages(currentUserHacker.id, selectedDmOtherParticipantId);
    } finally {
      setLoadingOlderDm(false);
    }
  }

  async function handleLoadOlderFaction() {
    if (!effectiveFactionId) return;
    suppressGroupAutoScrollRef.current = true;
    setLoadingOlderFaction(true);
    try {
      await loadOlderFactionMessages(effectiveFactionId);
    } finally {
      setLoadingOlderFaction(false);
    }
  }

  async function handleSendDm(e: React.FormEvent) {
    e.preventDefault();
    const body = dmMessageText.trim();
    if (!body || !currentUserHacker || !selectedDmOtherParticipantId) return;
    setDmSendError(null);
    const result = await sendDm(currentUserHacker.id, selectedDmOtherParticipantId, body);
    if (result.success) {
      setDmMessageText("");
    } else {
      setDmSendError(mapHackerboardMessagingError(result.errorMessage, t));
    }
  }

  async function handleSendGroupMessage(e: React.FormEvent) {
    e.preventDefault();
    const body = groupMessageText.trim();
    if (!body || !currentUserHacker || !effectiveFactionId) return;
    setGroupSendError(null);
    const result = await sendFactionGroupMessage(effectiveFactionId, currentUserHacker.id, body);
    if (result.success) {
      setGroupMessageText("");
    } else {
      setGroupSendError(mapHackerboardMessagingError(result.errorMessage, t));
    }
  }

  async function handleGroupUsernameInvite(e: React.FormEvent) {
    e.preventDefault();
    if (!currentUserHacker) return;
    setGroupInviteFeedback(null);
    setGroupInviteBusy(true);
    try {
      const r = await sendFactionInviteByUsername(groupInviteUsername);
      if (!r.success) {
        setGroupInviteFeedback(mapFactionInviteUiError(r.errorMessage, t));
      } else {
        setGroupInviteUsername("");
      }
    } finally {
      setGroupInviteBusy(false);
    }
  }

  async function handleCancelOutgoingInvite(inviteId: string) {
    setGroupInviteFeedback(null);
    setCancelOutgoingBusyId(inviteId);
    try {
      const r = await cancelFactionInviteOutgoing(inviteId);
      if (!r.success) {
        setGroupInviteFeedback(mapFactionInviteUiError(r.errorMessage, t));
      }
    } finally {
      setCancelOutgoingBusyId(null);
    }
  }

  function openProfile(userId: string) {
    setProfileActionError(null);
    setSelectedProfileUserId(userId);
    setSection("profile");
  }

  function closeProfile() {
    setSelectedProfileUserId(null);
    setSection("feed");
  }

  function openProfileAndMessage(userId: string) {
    openConversation(userId);
    setSection("messages");
    setSelectedProfileUserId(null);
  }

  function openConversation(otherParticipantId: string) {
    if (!currentUserHacker) return;
    const conversationId = getConversationId(currentUserHacker.id, otherParticipantId);
    setSelectedDmConversationId(conversationId);
    setSelectedDmOtherParticipantId(otherParticipantId);
    setServerInviteActionError(null);
    setDmSendError(null);
  }

  function openServerFactionInvitesPanel() {
    setSelectedDmConversationId(SERVER_FACTION_INVITES_PANEL_ID);
    setSelectedDmOtherParticipantId(SERVER_FACTION_INVITES_PANEL_ID);
    setServerInviteActionError(null);
    setDmSendError(null);
  }

  const showProfileInviteToast = useCallback((message: string) => {
    if (profileInviteToastTimerRef.current !== null) {
      window.clearTimeout(profileInviteToastTimerRef.current);
    }
    setProfileInviteToast(message);
    profileInviteToastTimerRef.current = window.setTimeout(() => {
      setProfileInviteToast(null);
      profileInviteToastTimerRef.current = null;
    }, 4000);
  }, []);

  const { openFilePicker } = useFilePicker();

  const handleSetProfileAvatarFromVm = useCallback(() => {
    setProfileActionError(null);
    openFilePicker({
      mode: "file",
      initialPath: getDefaultInitialPath(),
      onSelect: (path) => {
        void (async () => {
          const r = await setHackerboardAvatarFromVmPath(path);
          if (!r.success) {
            setProfileActionError(r.errorMessage ?? t("feedError"));
          } else {
            showProfileInviteToast(t("profileAvatarUpdated"));
          }
        })();
      },
    });
  }, [openFilePicker, setHackerboardAvatarFromVmPath, showProfileInviteToast, t]);

  const handleSetFactionEmblemFromVm = useCallback(() => {
    setGroupInviteFeedback(null);
    openFilePicker({
      mode: "file",
      initialPath: getDefaultInitialPath(),
      onSelect: (path) => {
        void (async () => {
          const r = await setFactionEmblemFromVmPath(path);
          if (!r.success) {
            setGroupInviteFeedback(r.errorMessage ?? t("kickFactionFailed"));
          } else {
            showProfileInviteToast(t("factionEmblemUpdated"));
          }
        })();
      },
    });
  }, [openFilePicker, setFactionEmblemFromVmPath, showProfileInviteToast, t]);

  useEffect(() => {
    return () => {
      if (profileInviteToastTimerRef.current !== null) {
        window.clearTimeout(profileInviteToastTimerRef.current);
      }
    };
  }, []);

  return (
    <div className={styles.app}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarTitle}>{t("title")}</div>
        <button
          type="button"
          className={`${styles.navItem} ${section === "feed" ? styles.navItemActive : ""}`}
          onClick={() => setSection("feed")}
        >
          <span className={styles.navIcon}>
            <MessageSquare size={18} />
          </span>
          {t("feed")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "rankings" ? styles.navItemActive : ""}`}
          onClick={() => setSection("rankings")}
        >
          <span className={styles.navIcon}>
            <Trophy size={18} />
          </span>
          {t("rankings")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "messages" ? styles.navItemActive : ""}`}
          onClick={() => setSection("messages")}
        >
          <span className={styles.navIcon}>
            <Mail size={18} />
          </span>
          {t("messages")}
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "group" ? styles.navItemActive : ""}`}
          onClick={() => setSection("group")}
        >
          <span className={styles.navIcon}>
            <Users size={18} />
          </span>
          {t("group")}
        </button>
        {token && playerId ? (
          <button
            type="button"
            className={`${styles.navItem} ${
              section === "profile" && selectedProfileUserId === playerId ? styles.navItemActive : ""
            }`}
            onClick={() => openProfile(playerId)}
          >
            <span className={styles.navIcon}>
              <UserCircle size={18} />
            </span>
            {t("myProfile")}
          </button>
        ) : null}
      </aside>
      <main className={styles.main}>
        {token && rankingErrorDisplay ? (
          <div
            className={`${styles.rankingDegradedBar} ${
              rankingErrorDisplay.tone === "info"
                ? styles.rankingDegradedBarInfo
                : styles.rankingDegradedBarError
            }`}
            role="status"
          >
            <span>{rankingErrorDisplay.message}</span>
            <button type="button" className={styles.rankingRetryBtn} onClick={() => retryRanking()}>
              {t("retryRanking")}
            </button>
          </div>
        ) : null}
        {section === "feed" && (
          <div className={styles.feedArea} ref={feedAreaRef} onScroll={onFeedAreaScroll}>
            <div className={styles.feedHeader}>
              <h2 className={styles.feedTitle}>{t("feed")}</h2>
              <div className={styles.feedHeaderSpacer} aria-hidden />
              <div className={styles.feedHeaderActions}>
                <button
                  type="button"
                  className={styles.feedRefreshBtn}
                  onClick={() => refreshFeed()}
                  disabled={!token || feedLoading || feedLoadingMore}
                  aria-busy={feedRefreshing}
                  aria-label={t("refreshFeedAria")}
                  title={t("refreshFeed")}
                >
                  <span
                    className={feedRefreshing ? styles.feedRefreshIconSpin : undefined}
                    aria-hidden
                  >
                    <RotateCw size={18} aria-hidden />
                  </span>
                </button>
                <div className={styles.feedLangMenuWrap} ref={feedLangMenuRef}>
                <button
                  type="button"
                  className={`${styles.feedLangMenuBtn} ${feedLangMenuOpen ? styles.feedLangMenuBtnOpen : ""}`}
                  aria-expanded={feedLangMenuOpen}
                  aria-haspopup="listbox"
                  aria-label={t("feedLanguageMenuAria")}
                  onClick={() => setFeedLangMenuOpen((o) => !o)}
                >
                  <Globe size={18} aria-hidden />
                  {feedLanguageFilter !== "all" ? (
                    <span className={styles.feedLangMenuBadge} aria-hidden />
                  ) : null}
                </button>
                {feedLangMenuOpen ? (
                  <div className={styles.feedLangMenuPanel} role="listbox" aria-label={t("languageFilter")}>
                    {(
                      [
                        { value: "all" as const, label: t("langAll") },
                        { value: "en" as const, label: t("langEn") },
                        { value: "pt-br" as const, label: t("langPtBr") },
                      ] as const
                    ).map(({ value, label }) => (
                      <button
                        key={value}
                        type="button"
                        role="option"
                        aria-selected={feedLanguageFilter === value}
                        className={`${styles.feedLangMenuOption} ${
                          feedLanguageFilter === value ? styles.feedLangMenuOptionActive : ""
                        }`}
                        onClick={() => {
                          setFeedLanguageFilter(value);
                          setFeedLangMenuOpen(false);
                        }}
                      >
                        {label}
                      </button>
                    ))}
                  </div>
                ) : null}
                </div>
              </div>
            </div>
            {feedRefreshing ? (
              <div className={styles.feedRefreshProgressWrap} role="status" aria-live="polite">
                <div
                  className={styles.feedRefreshProgress}
                  role="progressbar"
                  aria-label={t("refreshFeedInProgress")}
                  aria-valuetext={t("refreshFeedInProgress")}
                />
                <span className={styles.feedRefreshProgressLabel}>{t("refreshFeedInProgress")}</span>
              </div>
            ) : null}
            <form className={styles.compose} onSubmit={(e) => void handlePostSubmit(e)}>
              {feedErrorDisplay ? (
                <p
                  className={
                    feedErrorDisplay.tone === "info" ? styles.feedErrorInfo : styles.feedError
                  }
                >
                  {feedErrorDisplay.message}
                </p>
              ) : null}
              {token ? (
                <>
                  <textarea
                    ref={composeTextareaRef}
                    className={styles.composeTextarea}
                    placeholder={t("whatsHappening")}
                    value={composeText}
                    onChange={(e) => setComposeText(e.target.value)}
                    rows={3}
                    aria-label="New post"
                  />
                  <div className={styles.composeActions}>
                    <label className={styles.composeLangInline} htmlFor="hackerboard-compose-lang">
                      <span className={styles.composeLangInlineLabel}>{t("composeLanguageShort")}</span>
                      <select
                        id="hackerboard-compose-lang"
                        className={styles.composeLangSelectCompact}
                        value={composePostLanguage}
                        onChange={(e) => setComposePostLanguage(e.target.value as FeedPostLanguage)}
                        aria-label={t("composeLanguage")}
                      >
                        <option value="en">{t("langEn")}</option>
                        <option value="pt-br">{t("langPtBr")}</option>
                      </select>
                    </label>
                    <button
                      type="submit"
                      className={styles.composeBtn}
                      disabled={!composeText.trim() || !currentUserHacker || feedLoading}
                    >
                      {t("post")}
                    </button>
                  </div>
                </>
              ) : (
                <p className={styles.emptyState}>{t("signInFeed")}</p>
              )}
            </form>
            {feedLoading ? (
              <div className={styles.loadingFeed}>
                <span className={styles.loadingFeedText}>{t("loadingFeed")}</span>
                <Loader2 size={28} className={styles.loadingFeedSpinner} aria-hidden />
              </div>
            ) : rootPosts.length === 0 ? (
              <p className={styles.emptyState}>{t("noPosts")}</p>
            ) : (
              <>
                {rootPosts.map((post) => {
                const authorHandle = getAuthorHandle(post, hackers);
                const authorHacker = post.authorId
                  ? hackers.find((x) => x.id === post.authorId)
                  : undefined;
                const authorAvatarUrl =
                  authorHacker?.avatarPixelB64 != null
                    ? pixelArtDataUrlFromNtpixelsBase64(authorHacker.avatarPixelB64)
                    : null;
                const likeCount = post.likeCount ?? 0;
                const isLiked = userLikedPostIds.has(post.id);
                const replies = repliesByRootId.get(post.id) ?? [];
                const replyCount = replies.length;
                const isExpanded = expandedThreadId === post.id;
                return (
                  <article key={post.id} className={styles.postCard}>
                    <div className={styles.postRow}>
                      {post.authorId ? (
                        <button
                          type="button"
                          className={`${styles.postIcon} ${styles.postIconUser} ${styles.postIconLink}`}
                          onClick={() => openProfile(post.authorId!)}
                          title={t("viewAuthorProfile", { handle: authorHandle })}
                          aria-label={t("viewAuthorProfile", { handle: authorHandle })}
                        >
                          {authorAvatarUrl ? (
                            <img
                              src={authorAvatarUrl}
                              alt=""
                              width={16}
                              height={16}
                              className={styles.postAuthorAvatar}
                            />
                          ) : (
                            <User size={16} />
                          )}
                        </button>
                      ) : (
                        <PostIcon type={post.type} />
                      )}
                      <div className={styles.postContent}>
                        <p className={styles.postMeta}>
                          {post.authorId ? (
                            <>
                              <button
                                type="button"
                                className={styles.postAuthorLink}
                                onClick={() => openProfile(post.authorId!)}
                              >
                                {authorHandle}
                              </button>
                              {" · "}
                            </>
                          ) : null}
                          {post.language ? (
                            <>
                              <span className={styles.postLangBadge}>
                                {post.language === "pt-br" ? t("langBadge_pt") : t("langBadge_en")}
                              </span>
                              {" · "}
                            </>
                          ) : null}
                          {formatTime(post.timestamp)}
                        </p>
                        <p className={styles.postBody}>{post.body}</p>
                        <div className={styles.postActions}>
                          <div className={styles.postActionsCluster}>
                            <button
                              type="button"
                              className={`${styles.postActionBtn} ${isExpanded ? styles.postActionBtnActive : ""}`}
                              onClick={() => toggleThread(post.id)}
                              title={t("comments")}
                              aria-label={t("comments")}
                            >
                              <MessageCircle size={16} />
                              <span>{replyCount > 0 ? replyCount : t("reply")}</span>
                            </button>
                            <button
                              type="button"
                              className={`${styles.postActionBtn} ${isLiked ? styles.postActionBtnLiked : ""}`}
                              onClick={() => {
                                triggerHeartAnim(post.id, isLiked ? "unlike" : "like");
                                void toggleLike(post.id, isLiked);
                              }}
                              title={isLiked ? t("unlike") : t("like")}
                              aria-label={isLiked ? t("unlike") : t("like")}
                            >
                              <span
                                className={
                                  heartAnimByPost[post.id] === "like"
                                    ? styles.heartIconAnimLike
                                    : heartAnimByPost[post.id] === "unlike"
                                      ? styles.heartIconAnimUnlike
                                      : styles.heartIconWrap
                                }
                              >
                                <Heart size={16} fill={isLiked ? "currentColor" : "none"} />
                              </span>
                              <span>{likeCount > 0 ? likeCount : t("like")}</span>
                            </button>
                          </div>
                          {token &&
                          post.authorId &&
                          currentUserHacker &&
                          post.authorId !== currentUserHacker.id ? (
                            <button
                              type="button"
                              className={styles.postActionReportBtn}
                              onClick={() => openReportModal(post)}
                              title={t("reportPostAria")}
                              aria-label={t("reportPostAria")}
                            >
                              <ShieldAlert size={16} aria-hidden />
                            </button>
                          ) : null}
                        </div>
                      </div>
                    </div>
                    {(replies.length > 0 || isExpanded) ? (
                      <div className={styles.thread}>
                        {replies.map((reply) => {
                          const replyAuthor = getAuthorHandle(reply, hackers);
                          return (
                            <div key={reply.id} className={styles.threadReply}>
                              <p className={styles.threadReplyMeta}>
                                {replyAuthor} · {formatTime(reply.timestamp)}
                              </p>
                              <p className={styles.threadReplyBody}>{reply.body}</p>
                            </div>
                          );
                        })}
                        {isExpanded ? (
                          <form
                            className={styles.threadReplyForm}
                            onSubmit={(e) => void handleThreadReplySubmit(e, post.id)}
                          >
                            {token ? (
                              <>
                                <textarea
                                  ref={expandedThreadId === post.id ? threadReplyInputRef : undefined}
                                  className={styles.threadReplyInput}
                                  placeholder={`Reply to @${authorHandle}...`}
                                  value={expandedThreadId === post.id ? threadReplyText : ""}
                                  onChange={(e) => setThreadReplyText(e.target.value)}
                                  rows={2}
                                  aria-label="Reply"
                                />
                                <button
                                  type="submit"
                                  className={styles.threadReplyBtn}
                                  disabled={
                                    !threadReplyText.trim() || !currentUserHacker || feedLoading || feedLoadingMore
                                  }
                                >
                                  {t("reply")}
                                </button>
                              </>
                            ) : (
                              <p className={styles.emptyState}>{t("signInFeed")}</p>
                            )}
                          </form>
                        ) : null}
                      </div>
                    ) : null}
                  </article>
                );
              })}
                <div
                  ref={feedLoadMoreSentinelRef}
                  className={styles.feedLoadMoreSentinel}
                  aria-hidden
                />
                {feedLoadingMore ? (
                  <div className={styles.feedLoadMoreRow}>
                    <span className={styles.feedLoadMoreText}>{t("loadingMoreFeed")}</span>
                    <Loader2 size={22} className={styles.feedLoadMoreSpinner} aria-hidden />
                  </div>
                ) : null}
              </>
            )}
          </div>
        )}
        {section === "rankings" && (
          <div className={styles.rankingsArea}>
            <div className={styles.rankingsTabs}>
              <button
                type="button"
                className={`${styles.rankTab} ${rankTab === "hackers" ? styles.rankTabActive : ""}`}
                onClick={() => setRankTab("hackers")}
              >
                {t("rankTabHackers")}
              </button>
              <button
                type="button"
                className={`${styles.rankTab} ${rankTab === "factions" ? styles.rankTabActive : ""}`}
                onClick={() => setRankTab("factions")}
              >
                {t("rankTabFactions")}
              </button>
            </div>
            <div className={styles.searchWrap}>
              <Search size={16} />
              <input
                type="text"
                className={styles.searchInput}
                placeholder={
                  rankTab === "hackers"
                    ? t("rankSearchHackersPlaceholder")
                    : t("rankSearchFactionsPlaceholder")
                }
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                aria-label={t("rankSearchAria")}
              />
            </div>
            {currentUserHacker && rankTab === "hackers" && (
              <div className={styles.youCard}>
                <span className={styles.youLabel}>{t("rankYouLabel")}</span>
                {(() => {
                  const u = currentUserHacker.avatarPixelB64
                    ? pixelArtDataUrlFromNtpixelsBase64(currentUserHacker.avatarPixelB64)
                    : null;
                  return u ? (
                    <img src={u} alt="" width={28} height={28} className={styles.rankRowAvatar} />
                  ) : null;
                })()}
                <span className={styles.youRank}>#{currentUserHacker.rank}</span>
                <span className={styles.youPoints}>{currentUserHacker.points.toLocaleString()} pts</span>
              </div>
            )}
            {currentUserHacker && rankTab === "factions" && (() => {
              const userFaction = currentUserFaction;
              return userFaction ? (
                <div className={styles.youCard}>
                  <span className={styles.youLabel}>{t("rankYourFactionLabel")}</span>
                  {userFaction.emblemPixelB64 ? (
                    (() => {
                      const eu = pixelArtDataUrlFromNtpixelsBase64(userFaction.emblemPixelB64);
                      return eu ? (
                        <img src={eu} alt="" width={28} height={28} className={styles.rankRowAvatar} />
                      ) : null;
                    })()
                  ) : null}
                  <span className={styles.youRank}>
                    #{userFaction.rank} — {userFaction.name}
                  </span>
                  <span className={styles.youPoints}>{userFaction.totalPoints.toLocaleString()} pts</span>
                </div>
              ) : (
                <div className={styles.youCard}>
                  <span className={styles.youLabel}>{t("rankYouLabel")}</span>
                  <span className={styles.youPoints}>{t("rankNoFaction")}</span>
                </div>
              );
            })()}
            <div className={styles.rankList}>
              {rankTab === "hackers" ? (
                filteredHackers.length === 0 ? (
                  <p className={styles.emptyState}>{t("rankNoHackersMatch")}</p>
                ) : (
                  filteredHackers.map((h) => {
                    const rowAvatar = h.avatarPixelB64
                      ? pixelArtDataUrlFromNtpixelsBase64(h.avatarPixelB64)
                      : null;
                    return (
                      <div
                        key={h.id}
                        role="button"
                        tabIndex={0}
                        className={`${styles.rankRow} ${styles.rankRowInteractive} ${
                          currentUserHacker?.id === h.id ? styles.rankRowYou : ""
                        }`}
                        onClick={() => openProfile(h.id)}
                        onKeyDown={(ev) => {
                          if (ev.key === "Enter" || ev.key === " ") {
                            ev.preventDefault();
                            openProfile(h.id);
                          }
                        }}
                      >
                        <span className={styles.rankNum}>{h.rank}</span>
                        {rowAvatar ? (
                          <img
                            src={rowAvatar}
                            alt=""
                            width={24}
                            height={24}
                            className={styles.rankRowAvatar}
                          />
                        ) : (
                          <span className={styles.rankRowAvatarPlaceholder} aria-hidden>
                            <User size={16} />
                          </span>
                        )}
                        <span className={styles.rankName}>{h.username}</span>
                        <span className={styles.rankPoints}>{h.points.toLocaleString()} pts</span>
                      </div>
                    );
                  })
                )
              ) : (
                filteredFactions.length === 0 ? (
                  <p className={styles.emptyState}>{t("rankNoFactionsMatch")}</p>
                ) : (
                  filteredFactions.map((f: FactionWithRank) => {
                    const facEmblem = f.emblemPixelB64
                      ? pixelArtDataUrlFromNtpixelsBase64(f.emblemPixelB64)
                      : null;
                    return (
                      <div key={f.id} className={styles.rankRow}>
                        <span className={styles.rankNum}>{f.rank}</span>
                        {facEmblem ? (
                          <img
                            src={facEmblem}
                            alt=""
                            width={24}
                            height={24}
                            className={styles.rankRowAvatar}
                          />
                        ) : null}
                        <span className={styles.rankName}>{f.name}</span>
                        <span className={styles.factionMembers}>
                          {t("rankMembersCount", { count: f.memberIds.length })}
                        </span>
                        <span className={styles.rankPoints}>{f.totalPoints.toLocaleString()} pts</span>
                      </div>
                    );
                  })
                )
              )}
            </div>
          </div>
        )}
        {section === "messages" && (
          <div className={styles.messagesArea}>
            <div className={styles.conversationList}>
              <div className={styles.messagesHeader}>
                <h2 className={styles.messagesTitle}>Messages</h2>
              </div>
              {token && clusterRankingActive && (
                <button
                  type="button"
                  className={`${styles.conversationItem} ${
                    selectedDmOtherParticipantId === SERVER_FACTION_INVITES_PANEL_ID
                      ? styles.conversationItemSelected
                      : ""
                  }`}
                  onClick={() => openServerFactionInvitesPanel()}
                >
                  <span className={styles.conversationItemName}>
                    {t("factionInvitesInbox")}
                    {factionInvitesIncoming.length > 0 ? ` (${factionInvitesIncoming.length})` : ""}
                  </span>
                </button>
              )}
              {dmConversationsList.map((item) => {
                const other = hackers.find((x) => x.id === item.otherParticipantId);
                const isSelected = selectedDmConversationId === item.conversationId;
                return (
                  <button
                    key={item.conversationId}
                    type="button"
                    className={`${styles.conversationItem} ${isSelected ? styles.conversationItemSelected : ""}`}
                    onClick={() => openConversation(item.otherParticipantId)}
                  >
                    <span className={styles.conversationItemName}>{other?.username ?? item.otherParticipantId}</span>
                  </button>
                );
              })}
              {!currentUserHacker && <p className={styles.emptyState}>{t("signInMessages")}</p>}
              {currentUserHacker && dmConversationsList.length === 0 && (
                <p className={styles.emptyState}>{t("noConversationsYet")}</p>
              )}
            </div>
            <div className={styles.conversationPane}>
              {selectedDmOtherParticipantId === SERVER_FACTION_INVITES_PANEL_ID ? (
                <>
                  <div className={styles.conversationPaneHeader}>
                    <span className={styles.conversationPaneTitle}>{t("factionInvitesInbox")}</span>
                  </div>
                  {serverInviteActionError ? (
                    <p className={styles.inviteFeedbackError}>{serverInviteActionError}</p>
                  ) : null}
                  <div className={styles.messageList} ref={dmMessageListRef}>
                    {factionInvitesIncoming.length === 0 ? (
                      <p className={styles.emptyState}>{t("factionInvitesEmpty")}</p>
                    ) : (
                      factionInvitesIncoming.map((inv) => (
                        <div key={inv.inviteId} className={styles.dmInviteCard}>
                          <p className={styles.dmInviteText}>
                            {t("factionInviteServerText", {
                              faction: inv.factionName,
                              from: inv.fromUsername,
                            })}
                          </p>
                          <div className={styles.dmInviteActions}>
                            <button
                              type="button"
                              className={styles.dmInviteAccept}
                              onClick={async () => {
                                const r = await acceptServerFactionInvite(inv.inviteId);
                                if (!r.success) {
                                  setServerInviteActionError(r.errorMessage ?? t("factionInviteActionFailed"));
                                } else {
                                  setServerInviteActionError(null);
                                }
                              }}
                            >
                              {t("acceptInvite")}
                            </button>
                            <button
                              type="button"
                              className={styles.dmInviteDecline}
                              onClick={async () => {
                                const r = await declineServerFactionInvite(inv.inviteId);
                                if (!r.success) {
                                  setServerInviteActionError(r.errorMessage ?? t("factionInviteActionFailed"));
                                } else {
                                  setServerInviteActionError(null);
                                }
                              }}
                            >
                              {t("declineInvite")}
                            </button>
                          </div>
                          <span className={styles.messageTime}>{formatTime(inv.createdAtMs)}</span>
                        </div>
                      ))
                    )}
                  </div>
                </>
              ) : !selectedDmOtherParticipantId ? (
                <p className={styles.conversationPlaceholder}>{t("selectConversation")}</p>
              ) : (
                <>
                  <div className={styles.conversationPaneHeader}>
                    <button
                      type="button"
                      className={styles.conversationPaneTitleBtn}
                      onClick={() => openProfile(selectedDmOtherParticipantId)}
                      aria-label={t("viewProfilePeerAria", {
                        name:
                          hackers.find((h) => h.id === selectedDmOtherParticipantId)?.username ??
                          selectedDmOtherParticipantId,
                      })}
                    >
                      {hackers.find((h) => h.id === selectedDmOtherParticipantId)?.username ??
                        selectedDmOtherParticipantId}
                    </button>
                    {currentUserHacker && token ? (
                      <button
                        type="button"
                        className={styles.dmHeaderBlockBtn}
                        onClick={() => {
                          void (async () => {
                            const uname = hackers.find((h) => h.id === selectedDmOtherParticipantId)?.username;
                            if (!uname) return;
                            setDmSendError(null);
                            const r = isBlockedByMe(selectedDmOtherParticipantId)
                              ? await unblockPlayer(uname)
                              : await blockPlayer(uname);
                            if (!r.success) {
                              setDmSendError(mapBlockActionError(r.errorMessage, t));
                            }
                          })();
                        }}
                        title={isBlockedByMe(selectedDmOtherParticipantId) ? t("unblockUser") : t("blockUser")}
                        aria-label={isBlockedByMe(selectedDmOtherParticipantId) ? t("unblockUser") : t("blockUser")}
                      >
                        <UserX size={18} aria-hidden />
                      </button>
                    ) : null}
                  </div>
                  {dmSendError ? <p className={styles.inviteFeedbackError}>{dmSendError}</p> : null}
                  {token &&
                  clusterRankingActive &&
                  selectedDmConversationId &&
                  hasMoreOlderDmMessages(selectedDmConversationId) ? (
                    <div className={styles.loadOlderRow}>
                      <button
                        type="button"
                        className={styles.loadOlderBtn}
                        onClick={() => void handleLoadOlderDm()}
                        disabled={loadingOlderDm}
                      >
                        {loadingOlderDm ? t("loadingOlderMessages") : t("loadOlderMessages")}
                      </button>
                    </div>
                  ) : null}
                  <div className={styles.messageList} ref={dmMessageListRef}>
                    {selectedDmMessages.map((msg) => {
                      const isFromSelf = msg.senderId === currentUserHacker?.id;
                      const senderName = hackers.find((h) => h.id === msg.senderId)?.username ?? msg.senderId;
                      const isInviteToMe =
                        msg.type === "faction_invite" && msg.senderId === selectedDmOtherParticipantId;
                      const inviteUnresolved = msg.type === "faction_invite" && msg.accepted !== true;

                      if (msg.type === "faction_invite" && inviteUnresolved && currentUserHacker && isInviteToMe) {
                        return (
                          <div key={msg.id} className={styles.dmInviteCard}>
                            <p className={styles.dmInviteText}>
                              Invited to join <strong>{msg.factionName ?? "Faction"}</strong>{" "}
                              {t("dmInviteBy")}{" "}
                              <button
                                type="button"
                                className={styles.dmInlineProfileLink}
                                onClick={() => openProfile(msg.senderId)}
                                aria-label={t("viewProfilePeerAria", { name: senderName })}
                              >
                                {senderName}
                              </button>
                              .
                            </p>
                            <div className={styles.dmInviteActions}>
                              <button
                                type="button"
                                className={styles.dmInviteAccept}
                                onClick={() => acceptFactionInvite(msg.id, currentUserHacker.id)}
                              >
                                {t("acceptInvite")}
                              </button>
                              <button
                                type="button"
                                className={styles.dmInviteDecline}
                                onClick={() => declineFactionInvite(msg.id)}
                              >
                                {t("declineInvite")}
                              </button>
                            </div>
                            <span className={styles.messageTime}>{formatTime(msg.timestamp)}</span>
                          </div>
                        );
                      }

                      return (
                        <div
                          key={msg.id}
                          className={`${styles.messageBubble} ${isFromSelf ? styles.messageBubbleSelf : ""}`}
                        >
                          {!isFromSelf ? (
                            <button
                              type="button"
                              className={styles.dmSenderNameBtn}
                              onClick={() => openProfile(msg.senderId)}
                              aria-label={t("viewProfilePeerAria", { name: senderName })}
                            >
                              {senderName}
                            </button>
                          ) : null}
                          {msg.type === "faction_invite" ? (
                            <p className={styles.messageBody}>
                              Invite to join {msg.factionName ?? "Faction"}.
                              {msg.accepted === true && " (Accepted)"}
                              {msg.accepted === false && " (Declined)"}
                            </p>
                          ) : (
                            <p className={styles.messageBody}>{msg.body}</p>
                          )}
                          <span className={styles.messageTime}>{formatTime(msg.timestamp)}</span>
                        </div>
                      );
                    })}
                  </div>
                  <form className={styles.messageInputWrap} onSubmit={handleSendDm}>
                    <input
                      type="text"
                      className={styles.messageInput}
                      placeholder="Type a message..."
                      value={dmMessageText}
                      onChange={(e) => setDmMessageText(e.target.value)}
                      aria-label="Message"
                    />
                    <button type="submit" className={styles.messageSendBtn} disabled={!dmMessageText.trim()}>
                      Send
                    </button>
                  </form>
                </>
              )}
            </div>
          </div>
        )}
        {section === "group" && (
          <div className={styles.groupArea}>
            {!currentUserHacker ? (
              <p className={styles.groupEmptyState}>{t("signInFaction")}</p>
            ) : !effectiveFactionId ? (
              <div className={styles.groupNoFaction}>
                <div className={styles.groupNoFactionCard}>
                  <div className={styles.groupNoFactionIcon}>
                    <Users size={48} />
                  </div>
                  <h2 className={styles.groupNoFactionTitle}>{t("noFactionYetTitle")}</h2>
                  <p className={styles.groupNoFactionText}>{t("noFactionYetBody")}</p>
                  <div className={styles.groupNoFactionFormWrap}>
                    <h3 className={styles.createFactionTitle}>{t("createFactionSectionTitle")}</h3>
                    <CreateFactionForm
                      onCreate={(name) => {
                        void createFaction(name, currentUserHacker.id);
                      }}
                    />
                  </div>
                  <p className={styles.groupNoFactionHint}>{t("noFactionHint")}</p>
                </div>
              </div>
            ) : (
              <GroupWithFaction
                currentUserHacker={currentUserHacker}
                currentUserFaction={currentUserFaction!}
                hackers={hackers}
                currentUserFactionGroupMessages={currentUserFactionGroupMessages}
                groupMessageListRef={groupMessageListRef}
                groupMessageText={groupMessageText}
                setGroupMessageText={setGroupMessageText}
                onSendGroupMessage={handleSendGroupMessage}
                inviteFeedback={groupInviteFeedback}
                groupSendError={groupSendError}
                showLoadOlderMessages={
                  !!token &&
                  clusterRankingActive &&
                  !!effectiveFactionId &&
                  hasMoreOlderFactionMessages(effectiveFactionId)
                }
                onLoadOlderMessages={() => void handleLoadOlderFaction()}
                loadOlderMessagesPending={loadingOlderFaction}
                loadOlderMessagesLabel={t("loadOlderMessages")}
                loadingOlderMessagesLabel={t("loadingOlderMessages")}
                inviteUsername={groupInviteUsername}
                setInviteUsername={setGroupInviteUsername}
                onSubmitUsernameInvite={(e) => void handleGroupUsernameInvite(e)}
                usernameInvitePending={groupInviteBusy}
                outgoingInvites={factionInvitesOutgoing}
                onCancelOutgoingInvite={(id) => void handleCancelOutgoingInvite(id)}
                cancelOutgoingBusyId={cancelOutgoingBusyId}
                clusterRankingActive={clusterRankingActive}
                factionEmblemDataUrl={factionEmblemDataUrl}
                showFactionEmblemVmButton={
                  !!token &&
                  clusterRankingActive &&
                  !!currentUserFaction?.creatorId &&
                  currentUserFaction.creatorId === currentUserHacker.id
                }
                onChooseFactionEmblemFromVm={handleSetFactionEmblemFromVm}
                onLeaveFaction={() => void leaveFaction(currentUserHacker.id)}
              />
            )}
          </div>
        )}
        {section === "profile" && selectedProfileUserId && (
          <div className={styles.profileArea}>
            <div className={styles.profileHeader}>
              <button type="button" className={styles.profileBack} onClick={closeProfile} aria-label={t("profileBack")}>
                <ArrowLeft size={20} />
                {t("profileBack")}
              </button>
            </div>
            {profileUser && (
              <>
                <div className={styles.profileInfo}>
                  <div className={styles.profileIdentityRow}>
                    {(() => {
                      const url = profileUser.avatarPixelB64
                        ? pixelArtDataUrlFromNtpixelsBase64(profileUser.avatarPixelB64)
                        : null;
                      return url ? (
                        <img
                          src={url}
                          alt=""
                          width={48}
                          height={48}
                          className={styles.profileAvatarImg}
                        />
                      ) : (
                        <span className={styles.profileAvatarPlaceholder} aria-hidden>
                          <User size={28} />
                        </span>
                      );
                    })()}
                    <div>
                      <h2 className={styles.profileUsername}>{profileUser.username}</h2>
                      <p className={styles.profileMeta}>
                        Rank #{profileUser.rank} · {profileUser.points.toLocaleString()} pts
                      </p>
                    </div>
                  </div>
                  {playerId &&
                    selectedProfileUserId === playerId &&
                    token &&
                    clusterRankingActive && (
                      <button type="button" className={styles.vmPixelFileBtn} onClick={handleSetProfileAvatarFromVm}>
                        {t("setAvatarFromVmFile")}
                      </button>
                    )}
                  {profileActionError ? (
                    <p className={styles.inviteFeedbackError} role="alert">
                      {profileActionError}
                    </p>
                  ) : null}
                  {currentUserHacker && selectedProfileUserId !== currentUserHacker.id && (
                    <div className={styles.profileActions}>
                      {!isBlockedByMe(selectedProfileUserId) ? (
                        <button
                          type="button"
                          className={styles.profileMessageBtn}
                          onClick={() => openProfileAndMessage(selectedProfileUserId)}
                        >
                          {t("messageUser")}
                        </button>
                      ) : null}
                      {canSendFactionInvite(currentUserHacker.id, selectedProfileUserId) ? (
                        <button
                          type="button"
                          className={styles.profileInviteBtn}
                          onClick={() => {
                            void (async () => {
                              setProfileActionError(null);
                              const r = await sendFactionInviteByUsername(profileUser.username);
                              if (!r.success) {
                                setProfileActionError(mapFactionInviteUiError(r.errorMessage, t));
                              } else {
                                showProfileInviteToast(
                                  t("profileInviteSentToast", { username: profileUser.username })
                                );
                              }
                            })();
                          }}
                        >
                          {t("inviteToMyFaction")}
                        </button>
                      ) : null}
                      <button
                        type="button"
                        className={
                          isBlockedByMe(profileUser.id) ? styles.profileUnblockBtn : styles.profileBlockBtn
                        }
                        disabled={profileBlockBusy}
                        onClick={() => {
                          void (async () => {
                            setProfileActionError(null);
                            setProfileBlockBusy(true);
                            try {
                              const r = isBlockedByMe(profileUser.id)
                                ? await unblockPlayer(profileUser.username)
                                : await blockPlayer(profileUser.username);
                              if (!r.success) {
                                setProfileActionError(mapBlockActionError(r.errorMessage, t));
                              }
                            } finally {
                              setProfileBlockBusy(false);
                            }
                          })();
                        }}
                      >
                        {isBlockedByMe(profileUser.id) ? t("unblockUser") : t("blockUser")}
                      </button>
                    </div>
                  )}
                </div>
                <div className={styles.profilePosts}>
                  <h3 className={styles.profilePostsTitle}>{t("profilePostsTitle")}</h3>
                  {profilePosts.length === 0 ? (
                    <p className={styles.emptyState}>{t("profileNoPosts")}</p>
                  ) : (
                    profilePosts.map((p) => (
                      <article key={p.id} className={styles.profilePostCard}>
                        <p className={styles.postMeta}>{formatTime(p.timestamp)}</p>
                        <p className={styles.postBody}>{p.body}</p>
                      </article>
                    ))
                  )}
                </div>
              </>
            )}
          </div>
        )}
      </main>
      <Modal
        open={reportModalPost !== null}
        onClose={closeReportModal}
        title={t("reportPostTitle")}
        secondaryButton={{ label: t("reportPostCancel"), onClick: closeReportModal }}
        primaryButton={{
          label: t("reportPostSubmit"),
          onClick: submitReport,
          disabled: !reportReason,
        }}
      >
        <p className={styles.reportPostModalIntro}>{t("reportPostIntro")}</p>
        <fieldset className={styles.reportPostModalFieldset}>
          <legend className={styles.reportPostModalLegend}>{t("reportPostReason")}</legend>
          <div className={styles.reportPostModalRadioList}>
            {REPORT_REASON_VALUES.map((v) => (
              <label key={v} className={styles.reportPostModalRadioRow}>
                <input
                  type="radio"
                  name="hackerboard-report-reason"
                  value={v}
                  checked={reportReason === v}
                  onChange={() => setReportReason(v)}
                />
                <span className={styles.reportPostModalRadioLabel}>{t(`reportReason_${v}`)}</span>
              </label>
            ))}
          </div>
        </fieldset>
      </Modal>
      {profileInviteToast ? (
        <div className={styles.hackerboardToast} role="status" aria-live="polite">
          {profileInviteToast}
        </div>
      ) : null}
    </div>
  );
}
