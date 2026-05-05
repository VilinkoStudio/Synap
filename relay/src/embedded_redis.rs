use std::net::SocketAddr;

use anyhow::Context;
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

#[derive(Debug)]
pub struct EmbeddedRedisHandle {
    listen_addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<anyhow::Result<()>>,
}

impl EmbeddedRedisHandle {
    pub async fn spawn(listen_addr: SocketAddr) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(listen_addr)
            .await
            .with_context(|| format!("failed to bind embedded redis listener on {listen_addr}"))?;
        let local_addr = listener
            .local_addr()
            .context("failed to resolve embedded redis local address")?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let task = tokio::spawn(async move {
            mini_redis::server::run(listener, async move {
                let _ = shutdown_rx.await;
            })
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))
        });

        Ok(Self {
            listen_addr: local_addr,
            shutdown_tx: Some(shutdown_tx),
            task,
        })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }
}

impl Drop for EmbeddedRedisHandle {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use mini_redis::client;
    use tokio::time::{Duration, sleep};

    use super::EmbeddedRedisHandle;

    #[tokio::test]
    async fn embedded_redis_supports_basic_set_get() -> anyhow::Result<()> {
        let handle = EmbeddedRedisHandle::spawn("127.0.0.1:0".parse()?).await?;
        let mut client = connect_with_retry(handle.listen_addr()).await?;

        client
            .set("relay:test:key", "value".into())
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let value = client
            .get("relay:test:key")
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))?
            .context("expected redis value to exist")?;

        assert_eq!(std::str::from_utf8(value.as_ref())?, "value");

        drop(handle);
        Ok(())
    }

    async fn connect_with_retry(addr: std::net::SocketAddr) -> anyhow::Result<client::Client> {
        let mut last_error = None;

        for _ in 0..10 {
            match client::connect(addr).await {
                Ok(client) => return Ok(client),
                Err(error) => {
                    last_error = Some(error);
                    sleep(Duration::from_millis(20)).await;
                }
            }
        }

        Err(anyhow::anyhow!(
            "failed to connect to embedded redis after retries: {}",
            last_error
                .map(|error| error.to_string())
                .unwrap_or_else(|| "unknown error".to_owned())
        ))
    }
}
