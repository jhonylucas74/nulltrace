//! Real-time mailbox notifications via tokio broadcast channels.
//! One broadcast channel per email address; capacity 64 messages.

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

use super::db::email_service::EmailRecord;

/// Broadcast channel map: email_address -> Sender<MailboxEvent>.
pub type MailboxHub = Arc<DashMap<String, broadcast::Sender<MailboxEvent>>>;

/// Events delivered to connected mailbox clients.
#[derive(Debug, Clone)]
pub enum MailboxEvent {
    NewEmail(EmailRecord),
    UnreadCount(i64),
}

/// Create a new empty MailboxHub.
pub fn new_hub() -> MailboxHub {
    Arc::new(DashMap::new())
}

/// Notify a recipient's mailbox about a new incoming email.
/// No-op if no clients are subscribed for that address.
pub fn notify_new_email(hub: &MailboxHub, to_address: &str, email: EmailRecord) {
    if let Some(tx) = hub.get(to_address) {
        let _ = tx.send(MailboxEvent::NewEmail(email));
    }
}

/// Notify subscribers of an updated unread count (e.g. after marking email read).
/// No-op if no clients are subscribed for that address.
pub fn notify_unread_count(hub: &MailboxHub, email_address: &str, count: i64) {
    if let Some(tx) = hub.get(email_address) {
        let _ = tx.send(MailboxEvent::UnreadCount(count));
    }
}

/// Subscribe to mailbox events for the given email address.
/// Creates the broadcast channel if it doesn't exist yet.
pub fn subscribe(hub: &MailboxHub, email_address: &str) -> broadcast::Receiver<MailboxEvent> {
    if let Some(tx) = hub.get(email_address) {
        return tx.subscribe();
    }
    // Not found; create and insert, then subscribe.
    let (tx, rx) = broadcast::channel(64);
    hub.insert(email_address.to_string(), tx);
    rx
}
