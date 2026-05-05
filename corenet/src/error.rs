use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("listener is already stopped")]
    ListenerStopped,
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("mDNS error: {0}")]
    Mdns(#[from] mdns_sd::Error),
}

#[derive(Debug, Error)]
pub enum SyncNetError {
    #[error(transparent)]
    Net(#[from] NetError),

    #[error(transparent)]
    Core(#[from] synap_core::ServiceError),
}
