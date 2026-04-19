use super::*;

impl SynapService {
    pub fn get_recent_sync_sessions(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<SyncSessionRecordDTO>, ServiceError> {
        let limit = limit.unwrap_or(10);
        let tx = self.db.begin_read()?;
        let reader = SyncStatsReader::new(&tx)?;
        let mut records = reader
            .all()
            .map_err(redb::Error::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(redb::Error::from)?;
        records.sort_by(|left, right| {
            right
                .finished_at_ms
                .cmp(&left.finished_at_ms)
                .then_with(|| right.started_at_ms.cmp(&left.started_at_ms))
        });
        records.truncate(limit);
        Ok(records
            .into_iter()
            .map(Self::sync_stats_record_to_dto)
            .collect())
    }

    pub fn export_share(&self, note_ids: &[String]) -> Result<Vec<u8>, ServiceError> {
        let note_ids = Self::parse_ids(note_ids)?;
        ShareService::new(self).export_bytes(&note_ids)
    }

    pub fn import_share(&self, bytes: &[u8]) -> Result<ShareStatsDTO, ServiceError> {
        ShareService::new(self)
            .import_bytes(bytes)
            .map(Self::share_stats_to_dto)
    }

    pub fn initiate_sync<T>(&self, transport: T) -> Result<SyncSessionDTO, ServiceError>
    where
        T: Read + Write + Send,
    {
        self.run_sync_session(transport, ServiceSyncRole::Initiator)
    }

    pub fn listen_sync<T>(&self, transport: T) -> Result<SyncSessionDTO, ServiceError>
    where
        T: Read + Write + Send,
    {
        self.run_sync_session(transport, ServiceSyncRole::Listener)
    }

    fn run_sync_session<T>(
        &self,
        transport: T,
        role: ServiceSyncRole,
    ) -> Result<SyncSessionDTO, ServiceError>
    where
        T: Read + Write + Send,
    {
        self.run_sync_session_with_options(transport, role, Default::default())
    }

    fn run_sync_session_with_options<T>(
        &self,
        transport: T,
        role: ServiceSyncRole,
        options: crypto::CryptoChannelOptions,
    ) -> Result<SyncSessionDTO, ServiceError>
    where
        T: Read + Write + Send,
    {
        let channel = {
            let tx = self.db.begin_read()?;
            let reader = CryptoReader::new(&tx)?;
            match role {
                ServiceSyncRole::Initiator => {
                    crypto::CryptoChannel::connect(transport, &reader, options.clone())
                }
                ServiceSyncRole::Listener => {
                    crypto::CryptoChannel::accept(transport, &reader, options)
                }
            }
        };

        let mut channel = match channel {
            Ok(channel) => channel,
            Err(crypto::CryptoChannelError::UntrustedPeer {
                public_key,
                fingerprint: _fingerprint,
            }) => return self.pending_trust_session(public_key),
            Err(crypto::CryptoChannelError::PeerIdentityMismatch {
                actual_public_key, ..
            }) => return self.pending_trust_session(actual_public_key),
            Err(err) => return Err(ServiceError::Other(anyhow::anyhow!(err))),
        };

        let peer = channel.peer().clone();
        let peer_identity = SyncPeerIdentity::from_authenticated_peer(&peer);
        let sync_service =
            SyncService::new(self, Default::default()).with_peer_identity(peer_identity);
        let stats = match role {
            ServiceSyncRole::Initiator => sync_service
                .sync_as_initiator(&mut channel)
                .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))?,
            ServiceSyncRole::Listener => sync_service
                .sync_as_responder(&mut channel)
                .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))?,
        };

        Ok(SyncSessionDTO {
            status: SyncStatusDTO::Completed,
            peer: Self::peer_to_dto(peer.trust_record),
            stats: Some(Self::sync_stats_to_dto(stats)),
        })
    }

    fn pending_trust_session(&self, public_key: [u8; 32]) -> Result<SyncSessionDTO, ServiceError> {
        let tx = self.db.begin_write()?;
        let writer = CryptoWriter::new(&tx);
        let record = crypto::remember_untrusted_public_key(&writer, public_key, None)?;
        tx.commit()?;
        Ok(SyncSessionDTO {
            status: SyncStatusDTO::PendingTrust,
            peer: Self::peer_to_dto(record),
            stats: None,
        })
    }
}
