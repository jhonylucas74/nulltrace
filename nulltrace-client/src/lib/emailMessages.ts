/**
 * Mock email messages for the in-game Mail app. Fictional senders and subjects only (no real brands).
 * Supports threads (conversations), folders (inbox/spam), and unread state.
 */

export type EmailFolder = "inbox" | "spam" | "trash";

export interface EmailMessage {
  id: string;
  threadId: string;
  from: string;
  subject: string;
  date: string;
  /** Unix ms for sorting (newest first). */
  timestamp: number;
  body: string;
  folder: EmailFolder;
  unread: boolean;
}

const ts = (y: number, m: number, d: number, h: number, min: number) =>
  new Date(y, m - 1, d, h, min).getTime();

/** Single messages (no thread). */
const singleMessages: EmailMessage[] = [
  {
    id: "1",
    threadId: "t1",
    from: "support@nulltrace.local",
    subject: "Welcome to Nulltrace",
    date: "Feb 5, 14:32",
    timestamp: ts(2025, 2, 5, 14, 32),
    body: "Welcome to the system. Your account is active. If you have any questions, reply to this message or check the docs at help.nulltrace.local.",
    folder: "inbox",
    unread: true,
  },
  {
    id: "2",
    threadId: "t2",
    from: "alerts@nulltrace.local",
    subject: "Scheduled maintenance",
    date: "Feb 4, 09:15",
    timestamp: ts(2025, 2, 4, 9, 15),
    body: "Maintenance window: Feb 6, 02:00–04:00 UTC. Services may be briefly unavailable. No action required.",
    folder: "inbox",
    unread: true,
  },
  {
    id: "3",
    threadId: "t3",
    from: "noreply@hub.local",
    subject: "Weekly digest",
    date: "Feb 3, 18:00",
    timestamp: ts(2025, 2, 3, 18, 0),
    body: "Your weekly digest is ready. New topics: 3. Unread: 7. Visit hub.local to view.",
    folder: "inbox",
    unread: false,
  },
  {
    id: "4",
    threadId: "t4",
    from: "system@nulltrace.local",
    subject: "Password expiry reminder",
    date: "Feb 1, 08:00",
    timestamp: ts(2025, 2, 1, 8, 0),
    body: "Your password will expire in 14 days. Please update it from Settings > Security when convenient.",
    folder: "inbox",
    unread: false,
  },
];

/** Thread example: email exchange with several replies (Meeting notes). */
const threadMeeting: EmailMessage[] = [
  {
    id: "5",
    threadId: "thread-meeting",
    from: "team@project.local",
    subject: "Meeting notes",
    date: "Feb 2, 11:42",
    timestamp: ts(2025, 2, 2, 11, 42),
    body: "Notes from today's sync: next milestone is set for end of week. Tasks are in the tracker. Let us know if you need anything.",
    folder: "inbox",
    unread: false,
  },
  {
    id: "6",
    threadId: "thread-meeting",
    from: "me@nulltrace.local",
    subject: "Re: Meeting notes",
    date: "Feb 2, 12:15",
    timestamp: ts(2025, 2, 2, 12, 15),
    body: "Thanks for the notes. I'll take the auth task and have a draft by Thursday.",
    folder: "inbox",
    unread: false,
  },
  {
    id: "7",
    threadId: "thread-meeting",
    from: "team@project.local",
    subject: "Re: Meeting notes",
    date: "Feb 2, 14:00",
    timestamp: ts(2025, 2, 2, 14, 0),
    body: "Sounds good. We'll slot the review for Friday morning. I've added you to the review doc.",
    folder: "inbox",
    unread: true,
  },
  {
    id: "8",
    threadId: "thread-meeting",
    from: "me@nulltrace.local",
    subject: "Re: Meeting notes",
    date: "Feb 2, 15:30",
    timestamp: ts(2025, 2, 2, 15, 30),
    body: "Perfect. I'll send the link once it's ready.",
    folder: "inbox",
    unread: false,
  },
  {
    id: "9",
    threadId: "thread-meeting",
    from: "team@project.local",
    subject: "Re: Meeting notes",
    date: "Feb 2, 16:00",
    timestamp: ts(2025, 2, 2, 16, 0),
    body: "Great, thanks. Talk then.",
    folder: "inbox",
    unread: false,
  },
];

/** A couple of deleted messages for Trash (in-game mock). */
const trashMessages: EmailMessage[] = [
  {
    id: "trash-1",
    threadId: "t-old",
    from: "noreply@promo.local",
    subject: "Old promo (deleted)",
    date: "Jan 28, 10:00",
    timestamp: ts(2025, 1, 28, 10, 0),
    body: "This message was deleted by the user.",
    folder: "trash",
    unread: false,
  },
  {
    id: "trash-2",
    threadId: "t-old2",
    from: "notifications@nulltrace.local",
    subject: "Reminder (deleted)",
    date: "Jan 27, 16:30",
    timestamp: ts(2025, 1, 27, 16, 30),
    body: "Deleted reminder.",
    folder: "trash",
    unread: false,
  },
];

/** Initial inbox + trash: single messages, thread, and trash, sorted by timestamp (newest first). */
export const MOCK_INBOX: EmailMessage[] = [
  ...singleMessages,
  ...threadMeeting,
  ...trashMessages,
].sort((a, b) => b.timestamp - a.timestamp);
