import { useState, useMemo, useRef, useEffect } from "react";
import { MessageSquare, Trophy, Search, Flag, CheckCircle, Info, User, MessageCircle, Heart, Mail, Users, ArrowLeft } from "lucide-react";
import { useAuth } from "../contexts/AuthContext";
import {
  useHackerboard,
  type FeedPost,
  type FactionWithRank,
  getConversationId,
} from "../contexts/HackerboardContext";
import styles from "./HackerboardApp.module.css";

type Section = "feed" | "rankings" | "messages" | "group" | "profile";
type RankTab = "hackers" | "factions";

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
        placeholder="Faction name"
        value={name}
        onChange={(e) => setName(e.target.value)}
        aria-label="Faction name"
      />
      <button type="submit" className={styles.createFactionBtn} disabled={!name.trim()}>
        Create
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
  onInviteMember,
  onLeaveFaction,
}: {
  currentUserHacker: { id: string };
  currentUserFaction: FactionWithRank;
  hackers: { id: string; username: string }[];
  currentUserFactionGroupMessages: { id: string; senderId: string; body: string; timestamp: number }[];
  groupMessageListRef: React.RefObject<HTMLDivElement | null>;
  groupMessageText: string;
  setGroupMessageText: (v: string) => void;
  onSendGroupMessage: (e: React.FormEvent) => void;
  onInviteMember: (userId: string) => void;
  onLeaveFaction: () => void;
}) {
  const [groupTab, setGroupTab] = useState<GroupTab>("chat");
  const [showInvitePicker, setShowInvitePicker] = useState(false);
  const [leaveConfirm, setLeaveConfirm] = useState(false);
  const inviteCandidates = hackers.filter((h) => !currentUserFaction.memberIds.includes(h.id));

  return (
    <>
      <div className={styles.groupTitle}>{currentUserFaction.name}</div>
      <div className={styles.groupTabs}>
        <button
          type="button"
          className={groupTab === "chat" ? styles.groupTabActive : styles.groupTab}
          onClick={() => setGroupTab("chat")}
        >
          Chat
        </button>
        <button
          type="button"
          className={groupTab === "members" ? styles.groupTabActive : styles.groupTab}
          onClick={() => setGroupTab("members")}
        >
          Members
        </button>
      </div>
      {groupTab === "chat" && (
        <>
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
              placeholder="Message the group..."
              value={groupMessageText}
              onChange={(e) => setGroupMessageText(e.target.value)}
              aria-label="Group message"
            />
            <button type="submit" className={styles.messageSendBtn} disabled={!groupMessageText.trim()}>
              Send
            </button>
          </form>
        </>
      )}
      {groupTab === "members" && (
        <div className={styles.membersArea}>
          <h3 className={styles.membersSectionTitle}>Invite member</h3>
          {showInvitePicker ? (
            <ul className={styles.invitePickerList}>
              {inviteCandidates.map((h) => (
                <li key={h.id}>
                  <button
                    type="button"
                    className={styles.invitePickerItem}
                    onClick={() => {
                      onInviteMember(h.id);
                      setShowInvitePicker(false);
                    }}
                  >
                    {h.username}
                  </button>
                </li>
              ))}
              {inviteCandidates.length === 0 && <p className={styles.emptyState}>No hackers to invite.</p>}
              <button type="button" className={styles.invitePickerCancel} onClick={() => setShowInvitePicker(false)}>
                Cancel
              </button>
            </ul>
          ) : (
            <button type="button" className={styles.inviteMemberBtn} onClick={() => setShowInvitePicker(true)}>
              Invite member
            </button>
          )}
          <h3 className={styles.membersSectionTitle}>
            Members {currentUserFaction.memberIds.length > 0 && `(${currentUserFaction.memberIds.length})`}
          </h3>
          <ul className={styles.memberList}>
            {currentUserFaction.memberIds.map((id) => {
              const h = hackers.find((x) => x.id === id);
              return <li key={id} className={styles.memberItem}>{h?.username ?? id}</li>;
            })}
          </ul>
          <div className={styles.leaveFactionWrap}>
            {leaveConfirm ? (
              <>
                <span className={styles.leaveConfirmText}>Leave this faction?</span>
                <button type="button" className={styles.leaveConfirmBtn} onClick={() => { onLeaveFaction(); setLeaveConfirm(false); }}>
                  Yes, leave
                </button>
                <button type="button" className={styles.leaveCancelBtn} onClick={() => setLeaveConfirm(false)}>
                  Cancel
                </button>
              </>
            ) : (
              <button type="button" className={styles.leaveFactionBtn} onClick={() => setLeaveConfirm(true)}>
                Leave faction
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
  const { username } = useAuth();
  const {
    hackers,
    factions,
    feed,
    searchHackers,
    searchFactions,
    addFeedPost,
    toggleLike,
    userLikedPostIds,
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
  } = useHackerboard();
  const [section, setSection] = useState<Section>("feed");
  const [rankTab, setRankTab] = useState<RankTab>("hackers");
  const [selectedProfileUserId, setSelectedProfileUserId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [composeText, setComposeText] = useState("");
  const [expandedThreadId, setExpandedThreadId] = useState<string | null>(null);
  const [threadReplyText, setThreadReplyText] = useState("");
  const [selectedDmConversationId, setSelectedDmConversationId] = useState<string | null>(null);
  const threadReplyInputRef = useRef<HTMLTextAreaElement>(null);
  const [selectedDmOtherParticipantId, setSelectedDmOtherParticipantId] = useState<string | null>(null);
  const [dmMessageText, setDmMessageText] = useState("");
  const [groupMessageText, setGroupMessageText] = useState("");
  const composeTextareaRef = useRef<HTMLTextAreaElement>(null);
  const dmMessageListRef = useRef<HTMLDivElement>(null);
  const groupMessageListRef = useRef<HTMLDivElement>(null);

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

  /** Only root posts (no replyToId) appear in the main feed. */
  const rootPosts = useMemo(() => feed.filter((p) => !p.replyToId), [feed]);

  /** Replies per root post id, sorted by time. */
  const repliesByRootId = useMemo(() => {
    const map = new Map<string, FeedPost[]>();
    feed.forEach((p) => {
      if (p.replyToId) {
        const list = map.get(p.replyToId) ?? [];
        list.push(p);
        map.set(p.replyToId, list);
      }
    });
    map.forEach((list) => list.sort((a, b) => a.timestamp - b.timestamp));
    return map;
  }, [feed]);

  function handlePostSubmit(e: React.FormEvent) {
    e.preventDefault();
    const body = composeText.trim();
    if (!body || !currentUserHacker) return;
    addFeedPost({ type: "user", body, authorId: currentUserHacker.id });
    setComposeText("");
  }

  function toggleThread(postId: string) {
    setExpandedThreadId((prev) => (prev === postId ? null : postId));
    setThreadReplyText("");
    if (expandedThreadId !== postId) {
      setTimeout(() => threadReplyInputRef.current?.focus(), 100);
    }
  }

  function handleThreadReplySubmit(e: React.FormEvent, rootPostId: string) {
    e.preventDefault();
    const body = threadReplyText.trim();
    if (!body || !currentUserHacker) return;
    addFeedPost({
      type: "user",
      body,
      authorId: currentUserHacker.id,
      replyToId: rootPostId,
    });
    setThreadReplyText("");
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
  const profileUser = useMemo(
    () => (selectedProfileUserId ? hackers.find((h) => h.id === selectedProfileUserId) ?? null : null),
    [selectedProfileUserId, hackers]
  );
  const profilePosts = useMemo(
    () =>
      selectedProfileUserId
        ? feed.filter((p) => p.authorId === selectedProfileUserId && !p.replyToId && p.type === "user")
        : [],
    [selectedProfileUserId, feed]
  );

  useEffect(() => {
    dmMessageListRef.current?.scrollTo({ top: dmMessageListRef.current.scrollHeight, behavior: "smooth" });
  }, [selectedDmMessages.length]);
  useEffect(() => {
    if (section !== "group" || !currentUserFactionGroupMessages.length) return;
    groupMessageListRef.current?.scrollTo({ top: groupMessageListRef.current.scrollHeight, behavior: "smooth" });
  }, [section, currentUserFactionGroupMessages.length]);

  function handleSendDm(e: React.FormEvent) {
    e.preventDefault();
    const body = dmMessageText.trim();
    if (!body || !currentUserHacker || !selectedDmOtherParticipantId) return;
    sendDm(currentUserHacker.id, selectedDmOtherParticipantId, body);
    setDmMessageText("");
  }

  function handleSendGroupMessage(e: React.FormEvent) {
    e.preventDefault();
    const body = groupMessageText.trim();
    if (!body || !currentUserHacker || !effectiveFactionId) return;
    sendFactionGroupMessage(effectiveFactionId, currentUserHacker.id, body);
    setGroupMessageText("");
  }

  function openProfile(userId: string) {
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
  }

  return (
    <div className={styles.app}>
      <aside className={styles.sidebar}>
        <div className={styles.sidebarTitle}>Hackerboard</div>
        <button
          type="button"
          className={`${styles.navItem} ${section === "feed" ? styles.navItemActive : ""}`}
          onClick={() => setSection("feed")}
        >
          <span className={styles.navIcon}>
            <MessageSquare size={18} />
          </span>
          Feed
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "rankings" ? styles.navItemActive : ""}`}
          onClick={() => setSection("rankings")}
        >
          <span className={styles.navIcon}>
            <Trophy size={18} />
          </span>
          Rankings
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "messages" ? styles.navItemActive : ""}`}
          onClick={() => setSection("messages")}
        >
          <span className={styles.navIcon}>
            <Mail size={18} />
          </span>
          Messages
        </button>
        <button
          type="button"
          className={`${styles.navItem} ${section === "group" ? styles.navItemActive : ""}`}
          onClick={() => setSection("group")}
        >
          <span className={styles.navIcon}>
            <Users size={18} />
          </span>
          Group
        </button>
      </aside>
      <main className={styles.main}>
        {section === "feed" && (
          <div className={styles.feedArea}>
            <div className={styles.feedHeader}>
              <h2 className={styles.feedTitle}>Feed</h2>
              <span className={styles.feedLive}>
                <span className={styles.feedLiveDot} />
                Live
              </span>
            </div>
            <form className={styles.compose} onSubmit={handlePostSubmit}>
              <textarea
                ref={composeTextareaRef}
                className={styles.composeTextarea}
                placeholder="What's happening?"
                value={composeText}
                onChange={(e) => setComposeText(e.target.value)}
                rows={3}
                aria-label="New post"
              />
              <div className={styles.composeActions}>
                <button type="submit" className={styles.composeBtn} disabled={!composeText.trim() || !currentUserHacker}>
                  Post
                </button>
              </div>
            </form>
            {rootPosts.length === 0 ? (
              <p className={styles.emptyState}>No posts yet.</p>
            ) : (
              rootPosts.map((post) => {
                const authorHandle = getAuthorHandle(post, hackers);
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
                          title={`View ${authorHandle}'s profile`}
                          aria-label={`View ${authorHandle}'s profile`}
                        >
                          <User size={16} />
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
                          {formatTime(post.timestamp)}
                        </p>
                        <p className={styles.postBody}>{post.body}</p>
                        <div className={styles.postActions}>
                          <button
                            type="button"
                            className={`${styles.postActionBtn} ${isExpanded ? styles.postActionBtnActive : ""}`}
                            onClick={() => toggleThread(post.id)}
                            title="Comments"
                            aria-label="Comments"
                          >
                            <MessageCircle size={16} />
                            <span>{replyCount > 0 ? replyCount : "Reply"}</span>
                          </button>
                          <button
                            type="button"
                            className={`${styles.postActionBtn} ${isLiked ? styles.postActionBtnLiked : ""}`}
                            onClick={() => toggleLike(post.id)}
                            title={isLiked ? "Unlike" : "Like"}
                            aria-label={isLiked ? "Unlike" : "Like"}
                          >
                            <Heart size={16} fill={isLiked ? "currentColor" : "none"} />
                            <span>{likeCount > 0 ? likeCount : "Like"}</span>
                          </button>
                        </div>
                      </div>
                    </div>
                    {isExpanded && (
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
                        <form
                          className={styles.threadReplyForm}
                          onSubmit={(e) => handleThreadReplySubmit(e, post.id)}
                        >
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
                            disabled={!threadReplyText.trim() || !currentUserHacker}
                          >
                            Reply
                          </button>
                        </form>
                      </div>
                    )}
                  </article>
                );
              })
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
                Hackers
              </button>
              <button
                type="button"
                className={`${styles.rankTab} ${rankTab === "factions" ? styles.rankTabActive : ""}`}
                onClick={() => setRankTab("factions")}
              >
                Factions
              </button>
            </div>
            <div className={styles.searchWrap}>
              <Search size={16} />
              <input
                type="text"
                className={styles.searchInput}
                placeholder={rankTab === "hackers" ? "Search hackers..." : "Search factions..."}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                aria-label="Search"
              />
            </div>
            {currentUserHacker && rankTab === "hackers" && (
              <div className={styles.youCard}>
                <span className={styles.youLabel}>You</span>
                <span className={styles.youRank}>#{currentUserHacker.rank}</span>
                <span className={styles.youPoints}>{currentUserHacker.points.toLocaleString()} pts</span>
              </div>
            )}
            {currentUserHacker && rankTab === "factions" && (() => {
              const userFaction = currentUserFaction;
              return userFaction ? (
                <div className={styles.youCard}>
                  <span className={styles.youLabel}>Your faction</span>
                  <span className={styles.youRank}>#{userFaction.rank} — {userFaction.name}</span>
                  <span className={styles.youPoints}>{userFaction.totalPoints.toLocaleString()} pts</span>
                </div>
              ) : (
                <div className={styles.youCard}>
                  <span className={styles.youLabel}>You</span>
                  <span className={styles.youPoints}>No faction</span>
                </div>
              );
            })()}
            <div className={styles.rankList}>
              {rankTab === "hackers" ? (
                filteredHackers.length === 0 ? (
                  <p className={styles.emptyState}>No hackers match your search.</p>
                ) : (
                  filteredHackers.map((h) => (
                    <div
                      key={h.id}
                      className={`${styles.rankRow} ${currentUserHacker?.id === h.id ? styles.rankRowYou : ""}`}
                    >
                      <span className={styles.rankNum}>{h.rank}</span>
                      <span className={styles.rankName}>{h.username}</span>
                      <span className={styles.rankPoints}>{h.points.toLocaleString()} pts</span>
                      <button
                        type="button"
                        className={styles.rankProfileBtn}
                        onClick={() => openProfile(h.id)}
                        title="View profile"
                        aria-label="View profile"
                      >
                        Profile
                      </button>
                    </div>
                  ))
                )
              ) : (
                filteredFactions.length === 0 ? (
                  <p className={styles.emptyState}>No factions match your search.</p>
                ) : (
                  filteredFactions.map((f: FactionWithRank) => (
                    <div
                      key={f.id}
                      className={styles.rankRow}
                    >
                      <span className={styles.rankNum}>{f.rank}</span>
                      <span className={styles.rankName}>{f.name}</span>
                      <span className={styles.factionMembers}>{f.memberIds.length} members</span>
                      <span className={styles.rankPoints}>{f.totalPoints.toLocaleString()} pts</span>
                    </div>
                  ))
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
              {!currentUserHacker && <p className={styles.emptyState}>Sign in to see messages.</p>}
              {currentUserHacker && dmConversationsList.length === 0 && (
                <p className={styles.emptyState}>No conversations yet.</p>
              )}
            </div>
            <div className={styles.conversationPane}>
              {!selectedDmOtherParticipantId ? (
                <p className={styles.conversationPlaceholder}>Select a conversation.</p>
              ) : (
                <>
                  <div className={styles.conversationPaneHeader}>
                    <span className={styles.conversationPaneTitle}>
                      {hackers.find((h) => h.id === selectedDmOtherParticipantId)?.username ?? selectedDmOtherParticipantId}
                    </span>
                  </div>
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
                              Invited to join <strong>{msg.factionName ?? "Faction"}</strong> by {senderName}.
                            </p>
                            <div className={styles.dmInviteActions}>
                              <button
                                type="button"
                                className={styles.dmInviteAccept}
                                onClick={() => acceptFactionInvite(msg.id, currentUserHacker.id)}
                              >
                                Accept
                              </button>
                              <button
                                type="button"
                                className={styles.dmInviteDecline}
                                onClick={() => declineFactionInvite(msg.id)}
                              >
                                Decline
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
                          {!isFromSelf && <span className={styles.messageSender}>{senderName}</span>}
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
              <p className={styles.groupEmptyState}>Sign in to access the group.</p>
            ) : !effectiveFactionId ? (
              <div className={styles.groupNoFaction}>
                <div className={styles.groupNoFactionCard}>
                  <div className={styles.groupNoFactionIcon}>
                    <Users size={48} />
                  </div>
                  <h2 className={styles.groupNoFactionTitle}>No faction yet</h2>
                  <p className={styles.groupNoFactionText}>
                    Create your own faction to chat with a team, or accept an invite sent to you in Messages.
                  </p>
                  <div className={styles.groupNoFactionFormWrap}>
                    <h3 className={styles.createFactionTitle}>Create a faction</h3>
                    <CreateFactionForm
                      onCreate={(name) => {
                        createFaction(name, currentUserHacker.id);
                      }}
                    />
                  </div>
                  <p className={styles.groupNoFactionHint}>
                    Invites from other hackers will appear as messages with Accept / Decline.
                  </p>
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
                onInviteMember={(userId) => sendFactionInvite(currentUserHacker.id, userId, currentUserFaction!.id)}
                onLeaveFaction={() => leaveFaction(currentUserHacker.id)}
              />
            )}
          </div>
        )}
        {section === "profile" && selectedProfileUserId && (
          <div className={styles.profileArea}>
            <div className={styles.profileHeader}>
              <button type="button" className={styles.profileBack} onClick={closeProfile} aria-label="Back">
                <ArrowLeft size={20} />
                Back
              </button>
            </div>
            {profileUser && (
              <>
                <div className={styles.profileInfo}>
                  <h2 className={styles.profileUsername}>{profileUser.username}</h2>
                  <p className={styles.profileMeta}>
                    Rank #{profileUser.rank} · {profileUser.points.toLocaleString()} pts
                  </p>
                  {currentUserHacker && selectedProfileUserId !== currentUserHacker.id && (
                    <button
                      type="button"
                      className={styles.profileMessageBtn}
                      onClick={() => openProfileAndMessage(selectedProfileUserId)}
                    >
                      Message
                    </button>
                  )}
                </div>
                <div className={styles.profilePosts}>
                  <h3 className={styles.profilePostsTitle}>Posts</h3>
                  {profilePosts.length === 0 ? (
                    <p className={styles.emptyState}>No posts yet.</p>
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
    </div>
  );
}
