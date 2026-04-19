use super::*;

impl SynapService {
    pub fn get_local_identity(&self) -> Result<LocalIdentityDTO, ServiceError> {
        let tx = self.db.begin_read()?;
        let reader = CryptoReader::new(&tx)?;
        let identity_public_key = crypto::local_identity_public_key(&reader)?
            .ok_or_else(|| ServiceError::NotFound("local identity public key".into()))?;
        let signing_public_key = crypto::local_signing_public_key(&reader)?
            .ok_or_else(|| ServiceError::NotFound("local signing public key".into()))?;

        Ok(LocalIdentityDTO {
            identity: Self::public_key_info_to_dto(
                crypto::local_identity_key_id().to_string(),
                "x25519".into(),
                identity_public_key,
            ),
            signing: Self::public_key_info_to_dto(
                crypto::local_signing_key_id().to_string(),
                "ed25519".into(),
                signing_public_key,
            ),
        })
    }

    pub fn get_peers(&self) -> Result<Vec<PeerDTO>, ServiceError> {
        let tx = self.db.begin_read()?;
        let reader = CryptoReader::new(&tx)?;
        crypto::list_known_public_keys(&reader)
            .map(|records| records.into_iter().map(Self::peer_to_dto).collect())
            .map_err(Into::into)
    }

    pub fn trust_peer(
        &self,
        public_key: &[u8],
        note: Option<String>,
    ) -> Result<PeerDTO, ServiceError> {
        let public_key: [u8; 32] = public_key.try_into().map_err(|_| {
            ServiceError::Other(anyhow::anyhow!("peer public key must be 32 bytes"))
        })?;
        let tx = self.db.begin_write()?;
        let writer = CryptoWriter::new(&tx);
        let record = crypto::import_trusted_public_key(&writer, public_key, note)?;
        tx.commit()?;
        Ok(Self::peer_to_dto(record))
    }

    pub fn update_peer_note(
        &self,
        peer_id: &str,
        note: Option<String>,
    ) -> Result<PeerDTO, ServiceError> {
        let peer_id = Self::parse_id(peer_id)?;
        let tx = self.db.begin_write()?;
        let writer = CryptoWriter::new(&tx);
        let record = crypto::update_trusted_public_key_note(&writer, peer_id, note)?
            .ok_or(ServiceError::NotFound(peer_id.to_string()))?;
        tx.commit()?;
        Ok(Self::peer_to_dto(record))
    }

    pub fn set_peer_status(
        &self,
        peer_id: &str,
        status: PeerTrustStatusDTO,
    ) -> Result<PeerDTO, ServiceError> {
        let peer_id = Self::parse_id(peer_id)?;
        let status = match status {
            PeerTrustStatusDTO::Pending => crate::models::crypto::KeyStatus::Pending,
            PeerTrustStatusDTO::Trusted => crate::models::crypto::KeyStatus::Active,
            PeerTrustStatusDTO::Retired => crate::models::crypto::KeyStatus::Retired,
            PeerTrustStatusDTO::Revoked => crate::models::crypto::KeyStatus::Revoked,
        };
        let tx = self.db.begin_write()?;
        let writer = CryptoWriter::new(&tx);
        let record = crypto::update_trusted_public_key_status(&writer, peer_id, status)?
            .ok_or(ServiceError::NotFound(peer_id.to_string()))?;
        tx.commit()?;
        Ok(Self::peer_to_dto(record))
    }

    pub fn delete_peer(&self, peer_id: &str) -> Result<(), ServiceError> {
        let peer_id = Self::parse_id(peer_id)?;
        let tx = self.db.begin_write()?;
        let writer = CryptoWriter::new(&tx);
        if !crypto::delete_trusted_public_key(&writer, peer_id)? {
            return Err(ServiceError::NotFound(peer_id.to_string()));
        }
        tx.commit()?;
        Ok(())
    }
}
