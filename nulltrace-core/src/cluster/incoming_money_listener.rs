//! IncomingMoneyListener: Rust backend for listening to new incoming transactions.
//! Lua scripts register via incoming_money.listen(to_key) or listen_usd(token_path).
//! Poll loop finds new tx, sends to VM's channel. Lua recv() blocks until next tx.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use redis::AsyncCommands;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

/// Incoming transaction payload (sent to Lua).
#[derive(Debug, Clone)]
pub struct IncomingMoneyTx {
    pub id: Uuid,
    pub currency: String,
    pub from_key: String,
    pub to_key: String,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
}

const REDIS_KEY_PREFIX: &str = "incoming_money:";
/// TTL in seconds for Redis keys. Prevents unbounded growth if VMs stop reading.
const REDIS_KEY_TTL_SECS: u64 = 300;

pub struct IncomingMoneyListener {
    pool: PgPool,
    redis: redis::Client,
    startup_time: DateTime<Utc>,
    /// to_key -> vm_id (for poll: when we find tx for to_key, send to this vm)
    key_to_vm: DashMap<String, Uuid>,
    /// vm_id -> sender (poll loop uses this to send tx to the VM)
    vm_senders: DashMap<Uuid, mpsc::Sender<IncomingMoneyTx>>,
    /// vm_id -> receiver (Lua recv uses this; Arc+Mutex so we can recv without holding DashMap across await)
    vm_receivers: DashMap<Uuid, Arc<Mutex<mpsc::Receiver<IncomingMoneyTx>>>>,
}

impl IncomingMoneyListener {
    pub fn new(pool: PgPool, redis_url: &str) -> Result<Self, redis::RedisError> {
        let redis = redis::Client::open(redis_url)?;
        Ok(Self {
            pool,
            redis,
            startup_time: Utc::now(),
            key_to_vm: DashMap::new(),
            vm_senders: DashMap::new(),
            vm_receivers: DashMap::new(),
        })
    }

    /// Register to_key for this vm_id. One listener per key; replaces if exists.
    pub fn register(&self, to_key: String, vm_id: Uuid) {
        self.ensure_vm_channel(vm_id);
        self.key_to_vm.insert(to_key.clone(), vm_id);
        eprintln!("[incoming_money] registered to_key={} for vm_id={}", to_key, vm_id);
    }

    fn ensure_vm_channel(&self, vm_id: Uuid) {
        if !self.vm_senders.contains_key(&vm_id) {
            let (tx, rx) = mpsc::channel(64);
            self.vm_senders.insert(vm_id, tx);
            self.vm_receivers.insert(vm_id, Arc::new(Mutex::new(rx)));
        }
    }

    /// Block until next tx for this vm. Returns None if channel closed or vm not registered.
    pub async fn recv(&self, vm_id: Uuid) -> Option<IncomingMoneyTx> {
        let arc = self.vm_receivers.get(&vm_id).map(|r| Arc::clone(r.value()))?;
        let mut rx = arc.lock().await;
        rx.recv().await
    }

    /// Non-blocking: returns next tx if available, None otherwise. Does not block the VM tick.
    pub async fn try_recv(&self, vm_id: Uuid) -> Option<IncomingMoneyTx> {
        let arc = self.vm_receivers.get(&vm_id).map(|r| Arc::clone(r.value()))?;
        let mut rx = arc.lock().await;
        rx.try_recv().ok()
    }

    /// Spawn the poll loop. Runs forever.
    pub fn spawn_poll_loop(self: Arc<Self>, interval_ms: u64) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(interval_ms));
            loop {
                interval.tick().await;
                if let Err(e) = self.poll_once().await {
                    eprintln!("[incoming_money] poll error: {}", e);
                }
            }
        });
    }

    async fn poll_once(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let keys: Vec<String> = self.key_to_vm.iter().map(|r| r.key().clone()).collect();
        if keys.is_empty() {
            return Ok(());
        }

        let mut conn = self.redis.get_multiplexed_async_connection().await?;

        for to_key in keys {
            let vm_id = match self.key_to_vm.get(&to_key) {
                Some(v) => *v,
                None => continue,
            };

            let last_id: Option<String> = conn
                .get::<_, Option<String>>(format!("{}{}", REDIS_KEY_PREFIX, to_key))
                .await?;

            let last_id_for_refresh = last_id.clone();
            let use_id_filter = last_id.and_then(|s| s.parse::<Uuid>().ok());

            let rows: Vec<(Uuid, String, String, String, i64, DateTime<Utc>)> = if let Some(id) = use_id_filter {
                sqlx::query_as(
                    r#"
                    SELECT id, currency, from_key, to_key, amount, created_at
                    FROM wallet_transactions
                    WHERE to_key = $1 AND from_key != 'system' AND id > $2
                    ORDER BY created_at ASC
                    "#,
                )
                .bind(&to_key)
                .bind(id)
                .fetch_all(&self.pool)
                .await?
            } else {
                sqlx::query_as(
                    r#"
                    SELECT id, currency, from_key, to_key, amount, created_at
                    FROM wallet_transactions
                    WHERE to_key = $1 AND from_key != 'system' AND created_at >= $2
                    ORDER BY created_at ASC
                    "#,
                )
                .bind(&to_key)
                .bind(self.startup_time)
                .fetch_all(&self.pool)
                .await?
            };

            if !rows.is_empty() {
                eprintln!(
                    "[incoming_money] poll to_key={} found {} new tx (last_id={:?})",
                    to_key,
                    rows.len(),
                    use_id_filter
                );
            }

            let mut max_id: Option<Uuid> = None;
            let mut all_sent = true;
            if let Some(sender) = self.vm_senders.get(&vm_id) {
                for (id, currency, from_key, to_key_val, amount, created_at) in rows {
                    eprintln!(
                        "[incoming_money] sent tx id={} currency={} from={} to={} amount={}",
                        id, currency, from_key, to_key_val, amount
                    );
                    let tx = IncomingMoneyTx {
                        id,
                        currency,
                        from_key,
                        to_key: to_key_val,
                        amount,
                        created_at,
                    };
                    if sender.send(tx).await.is_err() {
                        eprintln!("[incoming_money] send to vm_id={} failed (channel closed?)", vm_id);
                        all_sent = false;
                        break;
                    }
                    max_id = Some(id);
                }
            }

            if all_sent {
                let value_to_store = max_id
                    .map(|id| id.to_string())
                    .or(last_id_for_refresh);
                if let Some(value) = value_to_store {
                    let redis_key = format!("{}{}", REDIS_KEY_PREFIX, to_key);
                    let _: () = redis::cmd("SET")
                        .arg(&redis_key)
                        .arg(&value)
                        .arg("EX")
                        .arg(REDIS_KEY_TTL_SECS)
                        .query_async(&mut conn)
                        .await?;
                }
            }
        }

        Ok(())
    }
}
