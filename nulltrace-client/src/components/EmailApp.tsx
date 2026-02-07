import { useState, useCallback, useMemo } from "react";
import type { EmailMessage, EmailFolder } from "../lib/emailMessages";
import { MOCK_INBOX } from "../lib/emailMessages";
import styles from "./EmailApp.module.css";

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

function formatDate(): string {
  const d = new Date();
  const mon = d.toLocaleString("en-GB", { month: "short" });
  const day = d.getDate();
  const time = d.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
  return `${mon} ${day}, ${time}`;
}

export default function EmailApp() {
  const [messages, setMessages] = useState<EmailMessage[]>(MOCK_INBOX);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [folder, setFolder] = useState<EmailFolder>("inbox");
  const [composing, setComposing] = useState(false);
  const [replyingTo, setReplyingTo] = useState<EmailMessage | null>(null);
  const [sentBanner, setSentBanner] = useState(false);
  const [composeTo, setComposeTo] = useState("");
  const [composeSubject, setComposeSubject] = useState("");
  const [composeBody, setComposeBody] = useState("");

  const filteredMessages = useMemo(
    () => messages.filter((m) => m.folder === folder).sort((a, b) => b.timestamp - a.timestamp),
    [messages, folder]
  );

  const selectedMessage = selectedId ? messages.find((m) => m.id === selectedId) : null;
  const threadMessages = selectedMessage
    ? messages
        .filter((m) => m.threadId === selectedMessage.threadId)
        .sort((a, b) => a.timestamp - b.timestamp)
    : [];

  const markAsRead = useCallback((id: string) => {
    setMessages((prev) =>
      prev.map((m) => (m.id === id ? { ...m, unread: false } : m))
    );
  }, []);

  const handleSelectMessage = useCallback(
    (id: string) => {
      setComposing(false);
      setReplyingTo(null);
      setSelectedId(id);
      markAsRead(id);
    },
    [markAsRead]
  );

  const handleCompose = useCallback(() => {
    setComposing(true);
    setReplyingTo(null);
    setSentBanner(false);
    setComposeTo("");
    setComposeSubject("");
    setComposeBody("");
  }, []);

  const handleReply = useCallback((msg: EmailMessage) => {
    setReplyingTo(msg);
    setComposing(true);
    setSentBanner(false);
    setComposeTo(msg.from);
    const subj = msg.subject.startsWith("Re:") ? msg.subject : `Re: ${msg.subject}`;
    setComposeSubject(subj);
    setComposeBody(`\n\n---\nOn ${msg.date}, ${msg.from} wrote:\n\n${msg.body}`);
  }, []);

  const handleSend = useCallback(() => {
    if (!composeTo.trim() || !composeSubject.trim()) return;
    const now = Date.now();
    const newMsg: EmailMessage = {
      id: `sent-${now}`,
      threadId: replyingTo ? replyingTo.threadId : `t-${now}`,
      from: "me@nulltrace.local",
      subject: composeSubject,
      date: formatDate(),
      timestamp: now,
      body: composeBody,
      folder: "inbox",
      unread: false,
    };
    setMessages((prev) => [newMsg, ...prev]);
    setComposing(false);
    setReplyingTo(null);
    setSelectedId(newMsg.id);
    setSentBanner(true);
    setTimeout(() => setSentBanner(false), 3000);
  }, [composeTo, composeSubject, composeBody, replyingTo]);

  const handleCancelCompose = useCallback(() => {
    setComposing(false);
    setReplyingTo(null);
  }, []);

  const handleMarkAsSpam = useCallback((id: string) => {
    setMessages((prev) =>
      prev.map((m) => (m.id === id ? { ...m, folder: "spam" as const } : m))
    );
    if (selectedId === id) setSelectedId(null);
  }, [selectedId]);

  const handleNotSpam = useCallback((id: string) => {
    setMessages((prev) =>
      prev.map((m) => (m.id === id ? { ...m, folder: "inbox" as const } : m))
    );
  }, []);

  const handleDelete = useCallback((id: string) => {
    setMessages((prev) =>
      prev.map((m) => (m.id === id ? { ...m, folder: "trash" as const } : m))
    );
    if (selectedId === id) setSelectedId(null);
  }, [selectedId]);

  const handleRestore = useCallback((id: string) => {
    setMessages((prev) =>
      prev.map((m) => (m.id === id ? { ...m, folder: "inbox" as const } : m))
    );
  }, []);

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
            {messages.filter((m) => m.folder === "inbox" && m.unread).length > 0 && (
              <span className={styles.unreadBadge}>
                {messages.filter((m) => m.folder === "inbox" && m.unread).length}
              </span>
            )}
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
                <th className={styles.colFrom}>From</th>
                <th className={styles.colSubject}>Subject</th>
                <th className={styles.colDate}>Date</th>
              </tr>
            </thead>
            <tbody>
              {filteredMessages.map((m) => (
                <tr
                  key={m.id}
                  className={
                    selectedId === m.id ? styles.selected : m.unread ? styles.unreadRow : undefined
                  }
                  onClick={() => handleSelectMessage(m.id)}
                >
                  <td>{m.from}</td>
                  <td>
                    <span className={styles.subjectCell}>
                      <span className={styles.subjectText}>{m.subject}</span>
                      {m.unread && <span className={styles.newTag}>New</span>}
                    </span>
                  </td>
                  <td>{m.date}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {filteredMessages.length === 0 && (
            <p className={styles.emptyFolder}>
              {folder === "trash"
                ? "No deleted messages."
                : folder === "spam"
                  ? "No spam messages."
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
              <div className={styles.composeToSubjectRow}>
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
                    <label className={styles.composeField}>
                      <span className={styles.composeLabel}>To</span>
                      <input
                        type="text"
                        value={composeTo}
                        onChange={(e) => setComposeTo(e.target.value)}
                        placeholder="recipient@example.local"
                        className={styles.composeInput}
                      />
                    </label>
                    <label className={styles.composeField}>
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
                  placeholder="Write your message..."
                />
              </label>
              <div className={styles.submitRow}>
                <button type="submit" className={styles.toolbarBtn}>
                  Send
                </button>
                <button type="button" className={styles.toolbarBtn} onClick={handleCancelCompose}>
                  Cancel
                </button>
              </div>
            </form>
          ) : selectedMessage ? (
            <>
              <div className={styles.threadHeader}>
                <h3 className={styles.threadSubject}>{threadMessages[0]?.subject ?? selectedMessage.subject}</h3>
                <div className={styles.panelActions}>
                  {folder !== "trash" && (
                    <button
                      type="button"
                      className={styles.actionBtn}
                      onClick={() => handleReply(threadMessages[threadMessages.length - 1] ?? selectedMessage)}
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
                    <button
                      type="button"
                      className={styles.actionBtn}
                      onClick={() => handleRestore(selectedMessage.id)}
                    >
                      Restore
                    </button>
                  )}
                </div>
              </div>
              <div className={styles.thread}>
                {threadMessages.map((msg) => (
                  <div key={msg.id} className={styles.threadMessage}>
                    <div className={styles.readMeta}>
                      <span className={styles.readFrom}>{msg.from}</span>
                      <span className={styles.readDate}>{msg.date}</span>
                    </div>
                    <div className={styles.readBody}>{msg.body}</div>
                    {threadMessages.length > 1 && (
                      <button
                        type="button"
                        className={styles.inlineReplyBtn}
                        onClick={() => handleReply(msg)}
                      >
                        Reply
                      </button>
                    )}
                  </div>
                ))}
              </div>
            </>
          ) : (
            <p className={styles.panelEmpty}>
              Select a message or click Compose. Use Inbox / Spam to filter.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
