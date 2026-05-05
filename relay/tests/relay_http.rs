use std::{net::SocketAddr, time::Duration};

use anyhow::Context;
use relay::{
    app::{AppState, AppStateParts},
    embedded_redis::EmbeddedRedisHandle,
    http::build_router,
    redis::RedisRuntime,
};
use synap_core::{
    service::SynapService,
    sync::{RelayHttpService, RelayInventory},
};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinHandle,
    time::{sleep, timeout},
};

struct TestRelayServer {
    base_url: String,
    _redis: EmbeddedRedisHandle,
    server_task: JoinHandle<()>,
}

impl TestRelayServer {
    async fn spawn() -> anyhow::Result<Self> {
        let redis = EmbeddedRedisHandle::spawn("127.0.0.1:0".parse()?).await?;
        let redis_url = format!("redis://{}", redis.listen_addr());
        let redis_runtime = RedisRuntime::new(redis_url)?;
        let state = AppState::from_parts(AppStateParts {
            server_name: "relay-test".to_owned(),
            redis_runtime,
            embedded_redis: None,
        });

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind relay test listener")?;
        let local_addr = listener.local_addr()?;
        let router = build_router(state);
        let server_task = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });

        wait_until_http_ready(local_addr).await?;

        Ok(Self {
            base_url: format!("http://{}", local_addr),
            _redis: redis,
            server_task,
        })
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Drop for TestRelayServer {
    fn drop(&mut self) {
        self.server_task.abort();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_round_trip_post_get_open_ack_then_empty() -> anyhow::Result<()> {
    let relay = TestRelayServer::spawn().await?;
    let sender = SynapService::open_memory()?;
    let recipient = SynapService::open_memory()?;

    let recipient_identity = recipient.get_local_identity()?;
    let recipient_mailbox_ed25519: [u8; 32] = recipient_identity
        .signing
        .public_key
        .as_slice()
        .try_into()
        .context("recipient signing public key should be 32 bytes")?;
    let recipient_x25519: [u8; 32] = recipient_identity
        .identity
        .public_key
        .as_slice()
        .try_into()
        .context("recipient identity public key should be 32 bytes")?;

    let envelope = sender.seal_relay_payload_for(recipient_x25519, b"relay integration payload")?;
    let sender_identity = sender.get_local_identity()?;
    let sender_ed25519: [u8; 32] = sender_identity
        .signing
        .public_key
        .as_slice()
        .try_into()
        .context("sender signing public key should be 32 bytes")?;

    let sender_client = RelayHttpService::new(&sender, relay.base_url());
    let recipient_client = RelayHttpService::new(&recipient, relay.base_url());

    sender_client.post_envelope(recipient_mailbox_ed25519, &envelope)?;

    let leased = recipient_client
        .get_and_open_for_local_recipient(recipient_mailbox_ed25519)?
        .context("expected leased envelope")?;
    assert_eq!(leased.sender_ed25519, sender_ed25519);
    assert_eq!(leased.payload, b"relay integration payload");
    assert!(!leased.lease_id.is_empty());
    assert!(leased.leased_until_ms > 0);

    recipient_client.ack_envelope(
        recipient_mailbox_ed25519,
        leased.sender_ed25519,
        &leased.lease_id,
    )?;

    let empty = recipient_client.get_envelope(recipient_mailbox_ed25519)?;
    assert!(empty.is_none());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_round_trip_supports_single_round_inventory_share_sync() -> anyhow::Result<()> {
    let relay = TestRelayServer::spawn().await?;
    let service_a = SynapService::open_memory()?;
    let service_b = SynapService::open_memory()?;

    let note_a = service_a.create_note("note from a".to_owned(), vec!["relay".to_owned()])?;
    let note_b = service_b.create_note("note from b".to_owned(), vec!["relay".to_owned()])?;

    let mailbox_a = service_a.local_relay_mailbox_public_key()?;
    let mailbox_b = service_b.local_relay_mailbox_public_key()?;
    let x25519_a = service_a.local_relay_recipient_identity_public_key()?;
    let x25519_b = service_b.local_relay_recipient_identity_public_key()?;

    let client_a = RelayHttpService::new(&service_a, relay.base_url());
    let client_b = RelayHttpService::new(&service_b, relay.base_url());

    let initial_inventory = service_a.build_relay_inventory()?;
    post_inventory_envelope(&service_a, &client_a, mailbox_b, x25519_b, &initial_inventory)?;

    let inventory_from_a = recv_inventory_and_ack(&service_b, &client_b, mailbox_b)?
        .context("service b should receive initial inventory")?;
    let share_for_a = service_b.export_relay_share_for_inventory(&inventory_from_a)?;
    let inventory_b = service_b.build_relay_inventory()?;
    let response = RelaySyncEnvelope {
        inventory: inventory_b,
        share: Some(share_for_a),
    };
    post_sync_envelope(&service_b, &client_b, mailbox_a, x25519_a, &response)?;

    let sync_from_b = recv_sync_envelope_and_ack(&service_a, &client_a, mailbox_a)?
        .context("service a should receive response inventory/share")?;
    let share_for_b = service_a.export_relay_share_for_inventory(&sync_from_b.inventory)?;
    service_a.import_share(sync_from_b.share.as_deref().context("missing share from b")?)?;
    if !share_for_b.is_empty() {
        let follow_up = RelaySyncEnvelope {
            inventory: service_a.build_relay_inventory()?,
            share: Some(share_for_b),
        };
        post_sync_envelope(&service_a, &client_a, mailbox_b, x25519_b, &follow_up)?;
    }

    let final_from_a = recv_sync_envelope_and_ack(&service_b, &client_b, mailbox_b)?
        .context("service b should receive follow-up share")?;
    service_b.import_share(final_from_a.share.as_deref().context("missing share from a")?)?;

    let notes_a = service_a.get_recent_note(None, Some(20))?;
    let notes_b = service_b.get_recent_note(None, Some(20))?;
    assert!(notes_a.iter().any(|note| note.id == note_a.id));
    assert!(notes_a.iter().any(|note| note.id == note_b.id));
    assert!(notes_b.iter().any(|note| note.id == note_a.id));
    assert!(notes_b.iter().any(|note| note.id == note_b.id));
    assert_eq!(notes_a.len(), notes_b.len());

    assert!(client_a.get_envelope(mailbox_a)?.is_none());
    assert!(client_b.get_envelope(mailbox_b)?.is_none());

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RelaySyncEnvelope {
    inventory: RelayInventory,
    #[serde(default)]
    share: Option<Vec<u8>>,
}

fn post_inventory_envelope(
    sender: &SynapService,
    client: &RelayHttpService<'_>,
    recipient_mailbox: [u8; 32],
    recipient_x25519: [u8; 32],
    inventory: &RelayInventory,
) -> anyhow::Result<()> {
    let body = postcard::to_allocvec(inventory)?;
    post_raw_envelope(sender, client, recipient_mailbox, recipient_x25519, &body)
}

fn post_sync_envelope(
    sender: &SynapService,
    client: &RelayHttpService<'_>,
    recipient_mailbox: [u8; 32],
    recipient_x25519: [u8; 32],
    payload: &RelaySyncEnvelope,
) -> anyhow::Result<()> {
    let body = postcard::to_allocvec(payload)?;
    post_raw_envelope(sender, client, recipient_mailbox, recipient_x25519, &body)
}

fn post_raw_envelope(
    sender: &SynapService,
    client: &RelayHttpService<'_>,
    recipient_mailbox: [u8; 32],
    recipient_x25519: [u8; 32],
    body: &[u8],
) -> anyhow::Result<()> {
    let envelope = sender.seal_relay_payload_for(recipient_x25519, body)?;
    client.post_envelope(recipient_mailbox, &envelope)?;
    Ok(())
}

fn recv_inventory_and_ack(
    _recipient: &SynapService,
    client: &RelayHttpService<'_>,
    mailbox: [u8; 32],
) -> anyhow::Result<Option<RelayInventory>> {
    let Some(leased) = client.get_and_open_for_local_recipient(mailbox)? else {
        return Ok(None);
    };
    let inventory = postcard::from_bytes(&leased.payload)?;
    client.ack_envelope(mailbox, leased.sender_ed25519, &leased.lease_id)?;
    Ok(Some(inventory))
}

fn recv_sync_envelope_and_ack(
    _recipient: &SynapService,
    client: &RelayHttpService<'_>,
    mailbox: [u8; 32],
) -> anyhow::Result<Option<RelaySyncEnvelope>> {
    let Some(leased) = client.get_and_open_for_local_recipient(mailbox)? else {
        return Ok(None);
    };
    let payload = postcard::from_bytes(&leased.payload)?;
    client.ack_envelope(mailbox, leased.sender_ed25519, &leased.lease_id)?;
    Ok(Some(payload))
}

async fn wait_until_http_ready(addr: SocketAddr) -> anyhow::Result<()> {
    for _ in 0..20 {
        if TcpStream::connect(addr).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(25)).await;
    }

    timeout(Duration::from_secs(1), async {
        loop {
            if TcpStream::connect(addr).await.is_ok() {
                break;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .context("relay test server did not become ready in time")?;

    Ok(())
}
