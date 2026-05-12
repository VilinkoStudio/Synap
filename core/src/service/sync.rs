use super::*;

impl SynapService {
    pub fn local_relay_mailbox_public_key(&self) -> Result<[u8; 32], ServiceError> {
        let tx = self.db.begin_read()?;
        let reader = CryptoReader::new(&tx)?;
        crypto::local_signing_public_key(&reader)?
            .ok_or_else(|| ServiceError::NotFound("local signing public key".into()))
    }

    pub fn local_relay_exchange_public_key(&self) -> Result<[u8; 32], ServiceError> {
        let tx = self.db.begin_read()?;
        let reader = CryptoReader::new(&tx)?;
        crypto::local_signing_exchange_public_key(&reader)?
            .ok_or_else(|| ServiceError::NotFound("local signing exchange public key".into()))
    }

    pub(crate) fn sign_relay_mailbox_auth(&self, payload: &[u8]) -> Result<[u8; 64], ServiceError> {
        let tx = self.db.begin_read()?;
        let reader = CryptoReader::new(&tx)?;
        crypto::sign_with_local_identity(&reader, payload)?.ok_or_else(|| {
            ServiceError::Other(anyhow::anyhow!("local signing identity is missing"))
        })
    }

    pub fn seal_relay_payload_for(
        &self,
        recipient_mailbox_ed25519_public_key: [u8; 32],
        payload: &[u8],
    ) -> Result<Vec<u8>, ServiceError> {
        let tx = self.db.begin_read()?;
        let reader = CryptoReader::new(&tx)?;
        let recipient_exchange_public_key =
            crypto::ed25519_public_key_to_x25519(recipient_mailbox_ed25519_public_key)?;
        crypto::seal_for_recipient(&reader, recipient_exchange_public_key, payload)
            .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))
    }

    pub fn build_relay_inventory(&self) -> Result<RelayInventory, ServiceError> {
        RelaySyncService::new(self)
            .build_local_inventory()
            .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))
    }

    pub fn export_relay_share_for_inventory(
        &self,
        remote_inventory: &RelayInventory,
    ) -> Result<Vec<u8>, ServiceError> {
        RelaySyncService::new(self).build_share_for_remote_inventory(remote_inventory)
    }

    pub fn relay_fetch_updates(
        &self,
        base_url: &str,
        api_key: Option<&str>,
    ) -> Result<RelayFetchStatsDTO, ServiceError> {
        let mut client = crate::sync::RelayHttpService::new(self, base_url);
        if let Some(api_key) = api_key {
            client = client.with_api_key(api_key);
        }
        client
            .fetch_relay_updates()
            .map(Self::relay_fetch_stats_to_dto)
            .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))
    }

    pub fn relay_push_updates(
        &self,
        base_url: &str,
        api_key: Option<&str>,
    ) -> Result<RelayPushStatsDTO, ServiceError> {
        let mut client = crate::sync::RelayHttpService::new(self, base_url);
        if let Some(api_key) = api_key {
            client = client.with_api_key(api_key);
        }
        client
            .push_relay_updates()
            .map(Self::relay_push_stats_to_dto)
            .map_err(|err| ServiceError::Other(anyhow::anyhow!(err)))
    }

    pub fn get_recent_sync_sessions(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<SyncSessionRecordDTO>, ServiceError> {
        let limit = limit.unwrap_or(10);
        let tx = self.db.begin_read()?;
        let reader = SyncStatsReader::new(&tx)?;
        let crypto_reader = CryptoReader::new(&tx)?;
        let records = reader.recent(limit)?;
        Ok(records
            .into_iter()
            .map(|record| {
                let peer =
                    crypto::get_known_public_key_by_bytes(&crypto_reader, record.peer_public_key)?;
                Ok::<SyncSessionRecordDTO, redb::Error>(Self::sync_stats_record_to_dto(
                    record, peer,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_peer_sync_stats(
        &self,
        peers_limit: Option<usize>,
        sessions_per_peer: Option<usize>,
    ) -> Result<Vec<PeerSyncStatsDTO>, ServiceError> {
        let peers_limit = peers_limit.unwrap_or(20);
        let sessions_per_peer = sessions_per_peer.unwrap_or(5);
        let tx = self.db.begin_read()?;
        let reader = SyncStatsReader::new(&tx)?;
        let crypto_reader = CryptoReader::new(&tx)?;
        let records = reader.grouped_by_peer(peers_limit, sessions_per_peer)?;

        Ok(records
            .into_iter()
            .map(|record| {
                let peer =
                    crypto::get_known_public_key_by_bytes(&crypto_reader, record.peer_public_key)?;
                Ok::<PeerSyncStatsDTO, redb::Error>(Self::peer_sync_stats_to_dto(record, peer))
            })
            .collect::<Result<Vec<_>, _>>()?)
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
