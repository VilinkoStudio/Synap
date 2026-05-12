use std::{
    collections::BTreeMap,
    net::SocketAddr,
    net::TcpStream,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, anyhow};
use bytes::Bytes;
use mini_redis::client;
use serde::{Deserialize, Serialize};
use tokio::{
    task,
    time::{Duration as TokioDuration, timeout},
};

const RELAY_STATE_KEY: &str = "relay:state";
const DEFAULT_RETENTION_TTL_MS: u64 = 7 * 24 * 60 * 60 * 1000;
const DEFAULT_LEASE_TTL_MS: u64 = 60 * 1000;

#[derive(Clone)]
pub struct RedisRuntime {
    url: String,
    addr: SocketAddr,
}

#[derive(Debug, Clone)]
pub struct RedisHealthStatus {
    pub status: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredEnvelope {
    pub sender_public_key_hex: String,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeasedEnvelope {
    pub sender_public_key_hex: String,
    pub lease_id: String,
    pub leased_until_ms: u64,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayStatusSnapshot {
    pub mailbox_count: usize,
    pub total_buffered_slots: usize,
    pub leased_slots: usize,
    pub total_delivered_count: u64,
    pub total_post_count: u64,
    pub total_ack_count: u64,
    pub total_expired_count: u64,
    pub total_replaced_count: u64,
    pub total_lease_grant_count: u64,
    pub total_lease_expire_count: u64,
    pub total_rejected_ack_count: u64,
    pub oldest_slot_age_ms: Option<u64>,
    pub newest_slot_age_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelayState {
    mailboxes: BTreeMap<String, BTreeMap<String, RelaySlotRecord>>,
    counters: RelayCounters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelaySlotRecord {
    updated_at_ms: u64,
    envelope_body_hex: String,
    lease: Option<RelayLeaseRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelayLeaseRecord {
    lease_id: String,
    leased_until_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RelayCounters {
    total_post_count: u64,
    total_ack_count: u64,
    total_delivered_count: u64,
    total_expired_count: u64,
    total_replaced_count: u64,
    total_lease_grant_count: u64,
    total_lease_expire_count: u64,
    total_rejected_ack_count: u64,
}

impl Default for RelayState {
    fn default() -> Self {
        Self {
            mailboxes: BTreeMap::new(),
            counters: RelayCounters::default(),
        }
    }
}

impl RedisRuntime {
    pub fn new(url: String) -> anyhow::Result<Self> {
        let addr = parse_redis_socket_addr(&url)?;
        Ok(Self { url, addr })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn mode_label(&self) -> &'static str {
        "external-or-embedded"
    }

    pub async fn health(&self) -> anyhow::Result<RedisHealthStatus> {
        let url = self.url.clone();
        let detail = timeout(
            TokioDuration::from_secs(2),
            task::spawn_blocking(move || redis_ping(&url)),
        )
        .await
        .with_context(|| format!("timed out while probing redis at {}", self.addr))?
        .context("redis probe task failed")??;

        Ok(RedisHealthStatus {
            status: "reachable",
            detail,
        })
    }

    pub async fn put_latest_slot(
        &self,
        recipient_public_key_hex: &str,
        envelope: StoredEnvelope,
    ) -> anyhow::Result<()> {
        let mut client = self.mailbox_client().await?;
        let mut state = load_state(&mut client).await?;
        let now_ms = now_ms();
        purge_expired_slots(&mut state, now_ms);

        let slots = state
            .mailboxes
            .entry(recipient_public_key_hex.to_owned())
            .or_default();

        if slots
            .insert(
                envelope.sender_public_key_hex,
                RelaySlotRecord {
                    updated_at_ms: now_ms,
                    envelope_body_hex: hex::encode(envelope.body),
                    lease: None,
                },
            )
            .is_some()
        {
            state.counters.total_replaced_count += 1;
        }

        state.counters.total_post_count += 1;
        store_state(&mut client, &state).await
    }

    pub async fn lease_next_slot(
        &self,
        recipient_public_key_hex: &str,
    ) -> anyhow::Result<Option<LeasedEnvelope>> {
        let mut client = self.mailbox_client().await?;
        let mut state = load_state(&mut client).await?;
        let now_ms = now_ms();
        purge_expired_slots(&mut state, now_ms);

        let leased = {
            let Some(slots) = state.mailboxes.get_mut(recipient_public_key_hex) else {
                store_state(&mut client, &state).await?;
                return Ok(None);
            };

            let mut selected_sender = None;
            for (sender_hex, slot) in slots.iter_mut() {
                let available = slot
                    .lease
                    .as_ref()
                    .is_none_or(|lease| lease.leased_until_ms <= now_ms);
                if available {
                    if slot
                        .lease
                        .as_ref()
                        .is_some_and(|lease| lease.leased_until_ms <= now_ms)
                    {
                        state.counters.total_lease_expire_count += 1;
                    }

                    let lease = RelayLeaseRecord {
                        lease_id: new_lease_id(now_ms, recipient_public_key_hex, sender_hex),
                        leased_until_ms: now_ms + DEFAULT_LEASE_TTL_MS,
                    };
                    let body = hex::decode(&slot.envelope_body_hex)
                        .context("failed to decode stored envelope body")?;
                    slot.lease = Some(lease.clone());
                    selected_sender = Some(LeasedEnvelope {
                        sender_public_key_hex: sender_hex.clone(),
                        lease_id: lease.lease_id,
                        leased_until_ms: lease.leased_until_ms,
                        body,
                    });
                    break;
                }
            }
            selected_sender
        };

        if leased.is_some() {
            state.counters.total_lease_grant_count += 1;
        }

        cleanup_empty_mailboxes(&mut state);
        store_state(&mut client, &state).await?;
        Ok(leased)
    }

    pub async fn ack_slot(
        &self,
        recipient_public_key_hex: &str,
        sender_public_key_hex: &str,
        lease_id: &str,
    ) -> anyhow::Result<bool> {
        let mut client = self.mailbox_client().await?;
        let mut state = load_state(&mut client).await?;
        let now_ms = now_ms();
        purge_expired_slots(&mut state, now_ms);

        let removed = if let Some(slots) = state.mailboxes.get_mut(recipient_public_key_hex) {
            let remove = slots
                .get(sender_public_key_hex)
                .and_then(|slot| slot.lease.as_ref())
                .is_some_and(|lease| lease.lease_id == lease_id);

            if remove {
                slots.remove(sender_public_key_hex);
                state.counters.total_ack_count += 1;
                state.counters.total_delivered_count += 1;
                true
            } else {
                state.counters.total_rejected_ack_count += 1;
                false
            }
        } else {
            state.counters.total_rejected_ack_count += 1;
            false
        };

        cleanup_empty_mailboxes(&mut state);
        store_state(&mut client, &state).await?;
        Ok(removed)
    }

    pub async fn status_snapshot(&self) -> anyhow::Result<RelayStatusSnapshot> {
        let mut client = self.mailbox_client().await?;
        let mut state = load_state(&mut client).await?;
        let now_ms = now_ms();
        purge_expired_slots(&mut state, now_ms);
        let snapshot = status_from_state(&state, now_ms);
        store_state(&mut client, &state).await?;
        Ok(snapshot)
    }

    async fn mailbox_client(&self) -> anyhow::Result<client::Client> {
        timeout(TokioDuration::from_secs(3), client::connect(self.addr))
            .await
            .with_context(|| format!("timed out while connecting to redis at {}", self.addr))?
            .map_err(|error| anyhow!("failed to connect to redis at {}: {error}", self.addr))
    }
}

fn status_from_state(state: &RelayState, now_ms: u64) -> RelayStatusSnapshot {
    let mut mailbox_count = 0usize;
    let mut total_buffered_slots = 0usize;
    let mut leased_slots = 0usize;
    let mut oldest_slot_age_ms = None;
    let mut newest_slot_age_ms = None;

    for slots in state.mailboxes.values() {
        if slots.is_empty() {
            continue;
        }
        mailbox_count += 1;
        total_buffered_slots += slots.len();

        for slot in slots.values() {
            if slot
                .lease
                .as_ref()
                .is_some_and(|lease| lease.leased_until_ms > now_ms)
            {
                leased_slots += 1;
            }

            let age = now_ms.saturating_sub(slot.updated_at_ms);
            oldest_slot_age_ms =
                Some(oldest_slot_age_ms.map_or(age, |current: u64| current.max(age)));
            newest_slot_age_ms =
                Some(newest_slot_age_ms.map_or(age, |current: u64| current.min(age)));
        }
    }

    RelayStatusSnapshot {
        mailbox_count,
        total_buffered_slots,
        leased_slots,
        total_delivered_count: state.counters.total_delivered_count,
        total_post_count: state.counters.total_post_count,
        total_ack_count: state.counters.total_ack_count,
        total_expired_count: state.counters.total_expired_count,
        total_replaced_count: state.counters.total_replaced_count,
        total_lease_grant_count: state.counters.total_lease_grant_count,
        total_lease_expire_count: state.counters.total_lease_expire_count,
        total_rejected_ack_count: state.counters.total_rejected_ack_count,
        oldest_slot_age_ms,
        newest_slot_age_ms,
    }
}

fn purge_expired_slots(state: &mut RelayState, now_ms: u64) {
    for slots in state.mailboxes.values_mut() {
        let before = slots.len();
        slots
            .retain(|_, slot| now_ms.saturating_sub(slot.updated_at_ms) < DEFAULT_RETENTION_TTL_MS);
        state.counters.total_expired_count += (before - slots.len()) as u64;
    }
    cleanup_empty_mailboxes(state);
}

fn cleanup_empty_mailboxes(state: &mut RelayState) {
    state.mailboxes.retain(|_, slots| !slots.is_empty());
}

fn new_lease_id(
    now_ms: u64,
    recipient_public_key_hex: &str,
    sender_public_key_hex: &str,
) -> String {
    let recipient_prefix_len = recipient_public_key_hex.len().min(8);
    let sender_prefix_len = sender_public_key_hex.len().min(8);
    format!(
        "{now_ms}-{}-{}",
        &recipient_public_key_hex[..recipient_prefix_len],
        &sender_public_key_hex[..sender_prefix_len]
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

async fn load_state(client: &mut client::Client) -> anyhow::Result<RelayState> {
    let value = client
        .get(RELAY_STATE_KEY)
        .await
        .map_err(|error| anyhow!("redis GET failed for key {RELAY_STATE_KEY}: {error}"))?;

    match value {
        Some(payload) => serde_json::from_slice::<RelayState>(&payload[..])
            .context("failed to decode relay state"),
        None => Ok(RelayState::default()),
    }
}

async fn store_state(client: &mut client::Client, state: &RelayState) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(state).context("failed to encode relay state")?;
    client
        .set(RELAY_STATE_KEY, Bytes::from(payload))
        .await
        .map_err(|error| anyhow!("redis SET failed for key {RELAY_STATE_KEY}: {error}"))?;
    Ok(())
}

fn connect_blocking_client(
    url: &str,
) -> anyhow::Result<redis_rs::connection::Connection<TcpStream>> {
    let (host, port) = parse_redis_host_port(url)?;
    redis_rs::connection::Connection::<TcpStream>::new_tcp(&host, port)
        .map_err(|error| anyhow!("failed to create redis client: {error:?}"))
}

fn parse_redis_socket_addr(url: &str) -> anyhow::Result<SocketAddr> {
    let without_scheme = url
        .strip_prefix("redis://")
        .ok_or_else(|| anyhow!("redis URL must start with redis://"))?;

    let authority = without_scheme
        .split('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| anyhow!("redis URL must contain host:port"))?;

    let host_port = authority
        .rsplit('@')
        .next()
        .ok_or_else(|| anyhow!("redis URL authority is invalid"))?;

    host_port
        .parse::<SocketAddr>()
        .with_context(|| format!("redis URL currently requires a socket address, got {host_port}"))
}

fn redis_ping(url: &str) -> anyhow::Result<String> {
    let mut client = connect_blocking_client(url)?;
    let response = client
        .ping()
        .map_err(|error| anyhow!("redis PING failed: {error:?}"))?;

    Ok(format!("redis PING returned {response:?}"))
}

fn parse_redis_host_port(url: &str) -> anyhow::Result<(String, u16)> {
    let without_scheme = url
        .strip_prefix("redis://")
        .ok_or_else(|| anyhow!("redis URL must start with redis://"))?;

    let authority = without_scheme
        .split('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| anyhow!("redis URL must contain host:port"))?;

    let host_port = authority
        .rsplit('@')
        .next()
        .ok_or_else(|| anyhow!("redis URL authority is invalid"))?;

    let (host, port) = host_port
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("redis URL must contain host:port"))?;

    let port = port
        .parse::<u16>()
        .with_context(|| format!("invalid redis port: {port}"))?;

    Ok((host.to_owned(), port))
}

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use super::{
        LeasedEnvelope, RedisRuntime, RelayStatusSnapshot, StoredEnvelope, parse_redis_host_port,
    };
    use crate::embedded_redis::EmbeddedRedisHandle;

    #[tokio::test]
    async fn health_check_succeeds_against_embedded_redis() -> anyhow::Result<()> {
        let handle = EmbeddedRedisHandle::spawn("127.0.0.1:0".parse()?).await?;
        let runtime = RedisRuntime::new(format!("redis://{}/", handle.listen_addr()))?;

        let status = runtime.health().await?;

        assert_eq!(status.status, "reachable");
        assert!(!status.detail.is_empty());

        drop(handle);
        Ok(())
    }

    #[tokio::test]
    async fn latest_slot_replaces_previous_sender_payload() -> anyhow::Result<()> {
        let handle = EmbeddedRedisHandle::spawn("127.0.0.1:0".parse()?).await?;
        let runtime = RedisRuntime::new(format!("redis://{}/", handle.listen_addr()))?;
        let recipient = "aa";
        let sender = "bb";

        runtime
            .put_latest_slot(
                recipient,
                StoredEnvelope {
                    sender_public_key_hex: sender.to_owned(),
                    body: b"old".to_vec(),
                },
            )
            .await?;
        runtime
            .put_latest_slot(
                recipient,
                StoredEnvelope {
                    sender_public_key_hex: sender.to_owned(),
                    body: b"new".to_vec(),
                },
            )
            .await?;

        let leased = runtime
            .lease_next_slot(recipient)
            .await?
            .context("expected leased envelope")?;

        assert_eq!(leased.sender_public_key_hex, sender);
        assert_eq!(leased.body, b"new");

        drop(handle);
        Ok(())
    }

    #[tokio::test]
    async fn ack_removes_only_matching_leased_slot() -> anyhow::Result<()> {
        let handle = EmbeddedRedisHandle::spawn("127.0.0.1:0".parse()?).await?;
        let runtime = RedisRuntime::new(format!("redis://{}/", handle.listen_addr()))?;
        let recipient = "aa";
        let sender = "bb";

        runtime
            .put_latest_slot(
                recipient,
                StoredEnvelope {
                    sender_public_key_hex: sender.to_owned(),
                    body: b"payload".to_vec(),
                },
            )
            .await?;

        let leased = runtime
            .lease_next_slot(recipient)
            .await?
            .context("expected leased envelope")?;

        assert!(!runtime.ack_slot(recipient, sender, "wrong-lease").await?);
        assert!(
            runtime
                .ack_slot(recipient, sender, &leased.lease_id)
                .await?
        );
        assert!(runtime.lease_next_slot(recipient).await?.is_none());

        drop(handle);
        Ok(())
    }

    #[tokio::test]
    async fn status_snapshot_counts_buffered_and_delivered_slots() -> anyhow::Result<()> {
        let handle = EmbeddedRedisHandle::spawn("127.0.0.1:0".parse()?).await?;
        let runtime = RedisRuntime::new(format!("redis://{}/", handle.listen_addr()))?;

        runtime
            .put_latest_slot(
                "r1",
                StoredEnvelope {
                    sender_public_key_hex: "s1".to_owned(),
                    body: b"a".to_vec(),
                },
            )
            .await?;
        runtime
            .put_latest_slot(
                "r1",
                StoredEnvelope {
                    sender_public_key_hex: "s2".to_owned(),
                    body: b"b".to_vec(),
                },
            )
            .await?;
        let leased = runtime
            .lease_next_slot("r1")
            .await?
            .context("expected leased envelope")?;
        let before_ack = runtime.status_snapshot().await?;
        assert_eq!(before_ack.mailbox_count, 1);
        assert_eq!(before_ack.total_buffered_slots, 2);
        assert_eq!(before_ack.leased_slots, 1);

        assert!(
            runtime
                .ack_slot("r1", &leased.sender_public_key_hex, &leased.lease_id)
                .await?
        );
        let after_ack = runtime.status_snapshot().await?;
        assert_eq!(after_ack.total_buffered_slots, 1);
        assert_eq!(after_ack.total_delivered_count, 1);
        assert_eq!(after_ack.total_ack_count, 1);

        drop(handle);
        Ok(())
    }

    #[test]
    fn parses_redis_host_port_from_url() -> anyhow::Result<()> {
        let (host, port) = parse_redis_host_port("redis://127.0.0.1:6380/")?;

        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 6380);

        Ok(())
    }

    #[allow(dead_code)]
    fn _assert_types(_: LeasedEnvelope, _: RelayStatusSnapshot) {}
}
