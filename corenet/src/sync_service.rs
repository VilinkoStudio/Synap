use std::sync::Arc;

use synap_core::{SynapService, SyncSessionDTO};

use crate::{
    spawn_incoming_loop, ConnectConfig, IncomingConnection, ListenConfig, ListenerState, NetError,
    SyncNetError, TcpListenerRuntime, TcpNetRuntime,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct SyncNetService {
    runtime: TcpNetRuntime,
}

impl SyncNetService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_listener_started(
        &self,
        config: ListenConfig,
    ) -> Result<TcpListenerRuntime, SyncNetError> {
        self.runtime.listen(config).map_err(Into::into)
    }

    pub fn connect_and_sync(
        &self,
        core: &SynapService,
        config: ConnectConfig,
    ) -> Result<SyncSessionDTO, SyncNetError> {
        let channel = self.runtime.connect(config)?;
        core.initiate_sync(channel).map_err(Into::into)
    }

    pub fn spawn_accept_loop<F>(
        &self,
        core: Arc<SynapService>,
        listener: TcpListenerRuntime,
        on_session: F,
    ) -> SyncAcceptLoopHandle
    where
        F: Fn(Result<SyncSessionDTO, SyncNetError>) + Send + Sync + 'static,
    {
        let callback = Arc::new(on_session);
        let incoming_loop = spawn_incoming_loop(listener, {
            let callback = Arc::clone(&callback);
            move |incoming| {
                let result = match incoming {
                    Ok(IncomingConnection { channel, .. }) => {
                        core.listen_sync(channel).map_err(SyncNetError::from)
                    }
                    Err(err) => Err(SyncNetError::Net(err)),
                };
                callback(result);
            }
        });

        SyncAcceptLoopHandle {
            incoming_loop: Some(incoming_loop),
        }
    }
}

pub struct SyncAcceptLoopHandle {
    incoming_loop: Option<crate::IncomingLoopHandle>,
}

impl SyncAcceptLoopHandle {
    pub fn state(&self) -> ListenerState {
        self.incoming_loop
            .as_ref()
            .map(crate::IncomingLoopHandle::state)
            .unwrap_or_default()
    }

    pub fn stop(&mut self) -> Result<(), NetError> {
        if let Some(mut incoming_loop) = self.incoming_loop.take() {
            incoming_loop.stop()?;
        }
        Ok(())
    }
}

impl Drop for SyncAcceptLoopHandle {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
