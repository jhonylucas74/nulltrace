import { useState, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
import type { EmailMessage } from "../contexts/GrpcContext";
import { useGrpc } from "../contexts/GrpcContext";
import { useEmail } from "../contexts/EmailContext";
import styles from "./EmailApp.module.css";

type EmailFolder = "inbox" | "sent" | "spam" | "trash";

function ComposeIcon() {
  return (
    <svg className={styles.toolbarIcon} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
      <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
    </svg>
  );
}
function InboxIcon() {
  return (
    <svg className={styles.toolbarIcon} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <polyline points="22 12 16 12 14 15 10 15 8 12 2 12" />
      <path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />
    </svg>
  );
}
function SentIcon() {
  return (
    <svg className={styles.toolbarIcon} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <line x1="22" y1="2" x2="11" y2="13" />
      <polygon points="22 2 15 22 11 13 2 9 22 2" />
    </svg>
  );
}
function SpamIcon() {
  return (
    <svg className={styles.toolbarIcon} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
      <line x1="12" y1="9" x2="12" y2="13" />
      <line x1="12" y1="17" x2="12.01" y2="17" />
    </svg>
  );
}
function TrashIcon() {
  return (
    <svg className={styles.toolbarIcon} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <polyline points="3 6 5 6 21 6" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      <line x1="10" y1="11" x2="10" y2="17" />
      <line x1="14" y1="11" x2="14" y2="17" />
    </svg>
  );
}

function formatTimestamp(ms: number): string {
  const d = new Date(ms);
  const mon = d.toLocaleString("en-GB", { month: "short" });
  const day = d.getDate();
  const time = d.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
  return `${mon} ${day}, ${time}`;
}

export default function EmailApp() {
  const { t } = useTranslation("email");
  const { t: tCommon } = useTranslation("common");
  const { emailAddress, mailToken, setUnreadCount, inboxInvalidated } = useEmail();
  const { getEmails, sendEmail, markEmailRead, moveEmail, deleteEmail } = useGrpc();

  const [messages, setMessages] = useState<EmailMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [folder, setFolder] = useState<EmailFolder>("inbox");
  const [pageByFolder, setPageByFolder] = useState<Record<EmailFolder, number>>({
    inbox: 0,
    sent: 0,
    spam: 0,
    trash: 0,
  });
  const [hasMoreByFolder, setHasMoreByFolder] = useState<Record<EmailFolder, boolean>>({
    inbox: true,
    sent: true,
    spam: true,
    trash: true,
  });
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [composing, setComposing] = useState(false);
  const [replyingTo, setReplyingTo] = useState<EmailMessage | null>(null);
  const [sentBanner, setSentBanner] = useState(false);
  const [composeTo, setComposeTo] = useState("");
  const [composeCc, setComposeCc] = useState("");
  const [composeBcc, setComposeBcc] = useState("");
  const [composeSubject, setComposeSubject] = useState("");
  const [composeBody, setComposeBody] = useState("");
  const [toDropdownOpen, setToDropdownOpen] = useState(false);
  const [toSearch, setToSearch] = useState("");
  const [showCc, setShowCc] = useState(false);
  const [showBcc, setShowBcc] = useState(false);

  const filteredMessages = messages
    .filter((m) => m.folder === folder)
    .sort((a, b) => b.sent_at_ms - a.sent_at_ms);

  const selectedMessage = selectedId ? messages.find((m) => m.id === selectedId) ?? null : null;

  // Valid contacts for To dropdown: unique addresses from inbox + sent (exclude spam/trash and self)
  const validContacts = (() => {
    const set = new Set<string>();
    for (const m of messages) {
      if (m.folder !== "inbox" && m.folder !== "sent") continue;
      if (m.from_address && m.from_address !== emailAddress) set.add(m.from_address);
      if (m.to_address && m.to_address !== emailAddress) set.add(m.to_address);
    }
    return Array.from(set).sort();
  })();
  const toFilteredContacts = toSearch.trim()
    ? validContacts.filter((c) =>
        c.toLowerCase().includes(toSearch.trim().toLowerCase())
      )
    : validContacts;

  const fetchMessages = useCallback(
    async (f: EmailFolder, page: number = 0) => {
      if (!emailAddress || !mailToken) return;
      if (page === 0) setLoading(true);
      try {
        const { emails, hasMore } = await getEmails(emailAddress, mailToken, f, page);
        setMessages((prev) => {
          const otherFolders = prev.filter((m) => m.folder !== f);
          if (page === 0) return [...otherFolders, ...emails];
          return [...prev, ...emails];
        });
        setHasMoreByFolder((prev) => ({ ...prev, [f]: hasMore }));
        setPageByFolder((prev) => ({ ...prev, [f]: page + 1 }));
        if (f === "inbox" && page === 0) {
          const unread = emails.filter((e) => !e.read).length;
          setUnreadCount(unread);
        }
      } catch (e) {
        console.error("[EmailApp] fetchMessages failed:", e);
      } finally {
        if (page === 0) setLoading(false);
      }
    },
    [emailAddress, mailToken, getEmails, setUnreadCount]
  );

  const loadMoreMessages = useCallback(() => {
    if (!emailAddress || !mailToken || loadingMore) return;
    const nextPage = pageByFolder[folder];
    setLoadingMore(true);
    getEmails(emailAddress, mailToken, folder, nextPage)
      .then(({ emails, hasMore }) => {
        setMessages((prev) => [...prev, ...emails]);
        setHasMoreByFolder((prev) => ({ ...prev, [folder]: hasMore }));
        setPageByFolder((prev) => ({ ...prev, [folder]: nextPage + 1 }));
      })
      .catch((e: unknown) => console.error("[EmailApp] loadMore failed:", e))
      .finally(() => setLoadingMore(false));
  }, [emailAddress, mailToken, folder, pageByFolder, loadingMore, getEmails]);

  // Fetch first page when folder or credentials change
  useEffect(() => {
    if (emailAddress && mailToken) {
      fetchMessages(folder, 0);
    }
  }, [folder, emailAddress, mailToken, fetchMessages]);

  // Real-time: when mailbox stream signals new email, refetch inbox first page only
  useEffect(() => {
    if (emailAddress && mailToken && inboxInvalidated > 0) {
      fetchMessages("inbox", 0);
    }
  }, [emailAddress, mailToken, inboxInvalidated, fetchMessages]);

  const handleSelectMessage = useCallback(
    (id: string) => {
      setComposing(false);
      setReplyingTo(null);
      setSelectedId(id);
      const msg = messages.find((m) => m.id === id);
      if (msg && !msg.read && emailAddress && mailToken) {
        markEmailRead(emailAddress, mailToken, id, true)
          .then(() => {
            setMessages((prev) =>
              prev.map((m) => (m.id === id ? { ...m, read: true } : m))
            );
            setUnreadCount((prev) => Math.max(0, prev - 1));
          })
          .catch(() => {});
      }
    },
    [messages, emailAddress, mailToken, markEmailRead, setUnreadCount]
  );

  const handleCompose = useCallback(() => {
    setComposing(true);
    setReplyingTo(null);
    setSentBanner(false);
    // Keep existing draft (do not clear); use Clear all to reset.
  }, []);

  const handleClearCompose = useCallback(() => {
    setComposeTo("");
    setComposeCc("");
    setComposeBcc("");
    setComposeSubject("");
    setComposeBody("");
    setReplyingTo(null);
    setToSearch("");
    setShowCc(false);
    setShowBcc(false);
  }, []);

  const handleReply = useCallback((msg: EmailMessage) => {
    setReplyingTo(msg);
    setComposing(true);
    setSentBanner(false);
    setComposeTo(msg.from_address);
    const subj = msg.subject.startsWith("Re:") ? msg.subject : `Re: ${msg.subject}`;
    setComposeSubject(subj);
    setComposeBody(`\n\n---\n${msg.from_address} wrote:\n\n${msg.body}`);
  }, []);

  const handleSend = useCallback(async () => {
    if (!composeTo.trim() || !composeSubject.trim() || !emailAddress || !mailToken) return;
    try {
      await sendEmail(
        emailAddress,
        mailToken,
        composeTo.trim(),
        composeSubject.trim(),
        composeBody,
        composeCc.trim() || undefined,
        composeBcc.trim() || undefined
      );
      setComposing(false);
      setReplyingTo(null);
      setSentBanner(true);
      setTimeout(() => setSentBanner(false), 3000);
      handleClearCompose();
      if (folder === "sent") {
        fetchMessages("sent");
      }
    } catch (e) {
      console.error("[EmailApp] sendEmail failed:", e);
    }
  }, [composeTo, composeCc, composeBcc, composeSubject, composeBody, emailAddress, mailToken, sendEmail, folder, fetchMessages, handleClearCompose]);

  const handleCancelCompose = useCallback(() => {
    setComposing(false);
    setReplyingTo(null);
  }, []);

  const handleMarkAsSpam = useCallback(
    async (id: string) => {
      if (!emailAddress || !mailToken) return;
      setMessages((prev) => prev.map((m) => (m.id === id ? { ...m, folder: "spam" } : m)));
      if (selectedId === id) setSelectedId(null);
      try {
        await moveEmail(emailAddress, mailToken, id, "spam");
      } catch {
        fetchMessages(folder);
      }
    },
    [emailAddress, mailToken, selectedId, moveEmail, fetchMessages, folder]
  );

  const handleNotSpam = useCallback(
    async (id: string) => {
      if (!emailAddress || !mailToken) return;
      setMessages((prev) => prev.map((m) => (m.id === id ? { ...m, folder: "inbox" } : m)));
      try {
        await moveEmail(emailAddress, mailToken, id, "inbox");
      } catch {
        fetchMessages(folder);
      }
    },
    [emailAddress, mailToken, moveEmail, fetchMessages, folder]
  );

  const handleDelete = useCallback(
    async (id: string) => {
      if (!emailAddress || !mailToken) return;
      setMessages((prev) => prev.map((m) => (m.id === id ? { ...m, folder: "trash" } : m)));
      if (selectedId === id) setSelectedId(null);
      try {
        await moveEmail(emailAddress, mailToken, id, "trash");
      } catch {
        fetchMessages(folder);
      }
    },
    [emailAddress, mailToken, selectedId, moveEmail, fetchMessages, folder]
  );

  const handleRestore = useCallback(
    async (id: string) => {
      if (!emailAddress || !mailToken) return;
      setMessages((prev) => prev.map((m) => (m.id === id ? { ...m, folder: "inbox" } : m)));
      try {
        await moveEmail(emailAddress, mailToken, id, "inbox");
      } catch {
        fetchMessages(folder);
      }
    },
    [emailAddress, mailToken, moveEmail, fetchMessages, folder]
  );

  const handlePermanentDelete = useCallback(
    async (id: string) => {
      if (!emailAddress || !mailToken) return;
      setMessages((prev) => prev.filter((m) => m.id !== id));
      if (selectedId === id) setSelectedId(null);
      try {
        await deleteEmail(emailAddress, mailToken, id);
      } catch {
        fetchMessages(folder);
      }
    },
    [emailAddress, mailToken, selectedId, deleteEmail, fetchMessages, folder]
  );

  const inboxUnread = messages.filter((m) => m.folder === "inbox" && !m.read).length;

  if (!emailAddress || !mailToken) {
    return (
      <div className={styles.app}>
        <div className={styles.loadingAccount}>
          <p className={styles.loadingAccountText}>Loading email account…</p>
          <Loader2 size={28} className={styles.loadingAccountSpinner} aria-hidden />
        </div>
      </div>
    );
  }

  return (
    <div className={styles.app}>
      <div className={styles.toolbar}>
        <button type="button" className={styles.toolbarBtn} onClick={handleCompose}>
          <ComposeIcon />
          <span>Compose</span>
        </button>
        <div className={styles.folderTabs}>
          <button
            type="button"
            className={folder === "inbox" ? styles.folderTabActive : styles.folderTab}
            onClick={() => setFolder("inbox")}
          >
            <InboxIcon />
            <span>Inbox</span>
            {inboxUnread > 0 && <span className={styles.unreadBadge}>{inboxUnread}</span>}
          </button>
          <button
            type="button"
            className={folder === "sent" ? styles.folderTabActive : styles.folderTab}
            onClick={() => setFolder("sent")}
          >
            <SentIcon />
            <span>Sent</span>
          </button>
          <button
            type="button"
            className={folder === "spam" ? styles.folderTabActive : styles.folderTab}
            onClick={() => setFolder("spam")}
          >
            <SpamIcon />
            <span>Spam</span>
            {messages.filter((m) => m.folder === "spam").length > 0 && (
              <span className={styles.folderCount}>
                {messages.filter((m) => m.folder === "spam").length}
              </span>
            )}
          </button>
          <button
            type="button"
            className={folder === "trash" ? styles.folderTabActive : styles.folderTab}
            onClick={() => setFolder("trash")}
          >
            <TrashIcon />
            <span>Trash</span>
            {messages.filter((m) => m.folder === "trash").length > 0 && (
              <span className={styles.folderCount}>
                {messages.filter((m) => m.folder === "trash").length}
              </span>
            )}
          </button>
        </div>
      </div>

      <div className={styles.main}>
        <div className={styles.tableWrap}>
          <table className={styles.table}>
            <thead>
              <tr>
                <th className={styles.colFrom}>{folder === "sent" ? "To" : "From"}</th>
                <th className={styles.colSubject}>Subject</th>
                <th className={styles.colDate}>Date</th>
              </tr>
            </thead>
            <tbody>
              {loading && (
                <tr>
                  <td colSpan={3} className={styles.emptyFolder}>
                    Loading…
                  </td>
                </tr>
              )}
              {!loading &&
                filteredMessages.map((m) => (
                  <tr
                    key={m.id}
                    className={
                      selectedId === m.id ? styles.selected : !m.read ? styles.unreadRow : undefined
                    }
                    onClick={() => handleSelectMessage(m.id)}
                  >
                    <td>{folder === "sent" ? m.to_address : m.from_address}</td>
                    <td>
                      <span className={styles.subjectCell}>
                        <span className={styles.subjectText}>{m.subject}</span>
                        {!m.read && folder === "inbox" && (
                          <span className={styles.newTag}>New</span>
                        )}
                      </span>
                    </td>
                    <td>{formatTimestamp(m.sent_at_ms)}</td>
                  </tr>
                ))}
              {!loading && hasMoreByFolder[folder] && (
                <tr>
                  <td colSpan={3} className={styles.loadMoreCell}>
                    <button
                      type="button"
                      className={styles.loadMoreButton}
                      onClick={loadMoreMessages}
                      disabled={loadingMore}
                    >
                      {loadingMore ? tCommon("loading") : t("load_more")}
                    </button>
                  </td>
                </tr>
              )}
            </tbody>
          </table>
          {!loading && filteredMessages.length === 0 && (
            <p className={styles.emptyFolder}>
              {folder === "trash"
                ? "No deleted messages."
                : folder === "spam"
                  ? "No spam messages."
                  : folder === "sent"
                    ? "No sent messages."
                    : "No messages."}
            </p>
          )}
        </div>

        <div className={styles.panel}>
          {sentBanner && <div className={styles.sentBanner}>Message sent.</div>}
          {composing ? (
            <form
              className={styles.composeForm}
              onSubmit={(e) => {
                e.preventDefault();
                handleSend();
              }}
            >
              <div className={styles.composeHeaderFields}>
                {replyingTo ? (
                  <>
                    <div className={styles.composeField}>
                      <span className={styles.composeLabel}>To</span>
                      <span className={styles.composeFixedValue}>{composeTo}</span>
                    </div>
                    <div className={styles.composeField}>
                      <span className={styles.composeLabel}>Subject</span>
                      <span className={styles.composeFixedValue}>{composeSubject}</span>
                    </div>
                  </>
                ) : (
                  <>
                    <div className={styles.composeFieldFull}>
                      <span className={styles.composeLabel}>To</span>
                      <div className={styles.composeToWrap}>
                        <input
                          type="text"
                          value={composeTo}
                          onChange={(e) => {
                            setComposeTo(e.target.value);
                            setToSearch(e.target.value);
                            setToDropdownOpen(true);
                          }}
                          onFocus={() => setToDropdownOpen(true)}
                          onBlur={() => setTimeout(() => setToDropdownOpen(false), 180)}
                          placeholder="recipient@mail.null or pick a contact"
                          className={styles.composeInput}
                          autoComplete="off"
                        />
                        {toDropdownOpen && toFilteredContacts.length > 0 && (
                          <ul className={styles.composeToDropdown} role="listbox">
                            {toFilteredContacts.slice(0, 8).map((addr) => (
                              <li
                                key={addr}
                                className={styles.composeToDropdownItem}
                                role="option"
                                onMouseDown={(e) => {
                                  e.preventDefault();
                                  setComposeTo(addr);
                                  setToSearch("");
                                  setToDropdownOpen(false);
                                }}
                              >
                                {addr}
                              </li>
                            ))}
                          </ul>
                        )}
                      </div>
                    </div>
                    <div className={styles.composeCcBccRow}>
                      <button
                        type="button"
                        className={styles.composeCcBccLink}
                        onClick={() => setShowCc(true)}
                        style={{ display: showCc ? "none" : undefined }}
                      >
                        CC
                      </button>
                      <button
                        type="button"
                        className={styles.composeCcBccLink}
                        onClick={() => setShowBcc(true)}
                        style={{ display: showBcc ? "none" : undefined }}
                      >
                        Cco
                      </button>
                    </div>
                    {showCc && (
                      <label className={styles.composeFieldFull}>
                        <span className={styles.composeLabel}>CC</span>
                        <input
                          type="text"
                          value={composeCc}
                          onChange={(e) => setComposeCc(e.target.value)}
                          placeholder="cc@mail.null (optional)"
                          className={styles.composeInput}
                        />
                      </label>
                    )}
                    {showBcc && (
                      <label className={styles.composeFieldFull}>
                        <span className={styles.composeLabel}>Cco (Bcc)</span>
                        <input
                          type="text"
                          value={composeBcc}
                          onChange={(e) => setComposeBcc(e.target.value)}
                          placeholder="bcc@mail.null (optional)"
                          className={styles.composeInput}
                        />
                      </label>
                    )}
                    <label className={styles.composeFieldFull}>
                      <span className={styles.composeLabel}>Subject</span>
                      <input
                        type="text"
                        value={composeSubject}
                        onChange={(e) => setComposeSubject(e.target.value)}
                        placeholder="Subject"
                        className={styles.composeInput}
                      />
                    </label>
                  </>
                )}
              </div>
              <label className={styles.composeMessageLabel}>
                <span className={styles.composeLabel}>Message</span>
                <textarea
                  className={styles.composeTextarea}
                  value={composeBody}
                  onChange={(e) => setComposeBody(e.target.value)}
                  placeholder="Write your message…"
                />
              </label>
              <div className={styles.submitRow}>
                <button type="submit" className={styles.toolbarBtn}>
                  Send
                </button>
                <button type="button" className={styles.toolbarBtn} onClick={handleCancelCompose}>
                  Cancel
                </button>
                <button type="button" className={styles.toolbarBtn} onClick={handleClearCompose}>
                  Clear all
                </button>
              </div>
            </form>
          ) : selectedMessage ? (
            <>
              <div className={styles.threadHeader}>
                <h3 className={styles.threadSubject}>{selectedMessage.subject}</h3>
                <div className={styles.panelActions}>
                  {folder !== "trash" && (
                    <button
                      type="button"
                      className={styles.actionBtn}
                      onClick={() => handleReply(selectedMessage)}
                    >
                      Reply
                    </button>
                  )}
                  {folder === "inbox" && (
                    <>
                      <button
                        type="button"
                        className={styles.actionBtnDanger}
                        onClick={() => handleMarkAsSpam(selectedMessage.id)}
                      >
                        Mark as spam
                      </button>
                      <button
                        type="button"
                        className={styles.actionBtn}
                        onClick={() => handleDelete(selectedMessage.id)}
                      >
                        Delete
                      </button>
                    </>
                  )}
                  {folder === "sent" && (
                    <button
                      type="button"
                      className={styles.actionBtn}
                      onClick={() => handlePermanentDelete(selectedMessage.id)}
                    >
                      Delete
                    </button>
                  )}
                  {folder === "spam" && (
                    <>
                      <button
                        type="button"
                        className={styles.actionBtn}
                        onClick={() => handleNotSpam(selectedMessage.id)}
                      >
                        Not spam
                      </button>
                      <button
                        type="button"
                        className={styles.actionBtn}
                        onClick={() => handleDelete(selectedMessage.id)}
                      >
                        Delete
                      </button>
                    </>
                  )}
                  {folder === "trash" && (
                    <>
                      <button
                        type="button"
                        className={styles.actionBtn}
                        onClick={() => handleRestore(selectedMessage.id)}
                      >
                        Restore
                      </button>
                      <button
                        type="button"
                        className={styles.actionBtnDanger}
                        onClick={() => handlePermanentDelete(selectedMessage.id)}
                      >
                        Delete forever
                      </button>
                    </>
                  )}
                </div>
              </div>
              <div className={styles.thread}>
                <div className={styles.threadMessage}>
                  <div className={styles.readMeta}>
                    <div className={styles.readMetaLines}>
                      <div className={styles.readMetaRow}>
                        <span className={styles.readMetaLabel}>From</span>
                        <span>{selectedMessage.from_address}</span>
                      </div>
                      <div className={styles.readMetaRow}>
                        <span className={styles.readMetaLabel}>To</span>
                        <span>{selectedMessage.to_address}</span>
                      </div>
                      {selectedMessage.cc_address && selectedMessage.cc_address.trim() !== "" && (
                        <div className={styles.readMetaRow}>
                          <span className={styles.readMetaLabel}>CC</span>
                          <span>{selectedMessage.cc_address}</span>
                        </div>
                      )}
                    </div>
                    <span className={styles.readDate}>{formatTimestamp(selectedMessage.sent_at_ms)}</span>
                  </div>
                  <div className={styles.readBody}>{selectedMessage.body}</div>
                </div>
              </div>
            </>
          ) : (
            <p className={styles.panelEmpty}>
              Select a message or click Compose.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
