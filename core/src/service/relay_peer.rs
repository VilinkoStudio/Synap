use super::*;

impl SynapService {
    pub fn get_relay_peer(
        &self,
        peer_public_key: &[u8],
    ) -> Result<Option<RelayPeerRecord>, ServiceError> {
        let peer_public_key: [u8; 32] = peer_public_key.try_into().map_err(|_| {
            ServiceError::Other(anyhow::anyhow!("relay peer public key must be 32 bytes"))
        })?;
        let tx = self.db.begin_read()?;
        let reader = RelayPeerReader::new(&tx)?;
        reader
            .get_by_public_key(&peer_public_key)
            .map_err(Into::into)
    }

    pub fn list_relay_peers(&self) -> Result<Vec<RelayPeerRecord>, ServiceError> {
        let tx = self.db.begin_read()?;
        let reader = RelayPeerReader::new(&tx)?;
        let iter = reader.all().map_err(redb::Error::from)?;
        iter.collect::<Result<Vec<_>, _>>()
            .map_err(redb::Error::from)
            .map_err(Into::into)
    }

    pub fn cache_relay_peer_inventory(
        &self,
        peer_public_key: &[u8],
        inventory: RelayInventory,
        cached_at_ms: u64,
    ) -> Result<RelayPeerRecord, ServiceError> {
        let peer_public_key: [u8; 32] = peer_public_key.try_into().map_err(|_| {
            ServiceError::Other(anyhow::anyhow!("relay peer public key must be 32 bytes"))
        })?;
        let tx = self.db.begin_write()?;
        let writer = RelayPeerWriter::new(&tx);
        let record = writer.put_cached_inventory(peer_public_key, inventory, cached_at_ms)?;
        tx.commit()?;
        Ok(record)
    }

    pub fn delete_relay_peer(&self, peer_public_key: &[u8]) -> Result<(), ServiceError> {
        let peer_public_key: [u8; 32] = peer_public_key.try_into().map_err(|_| {
            ServiceError::Other(anyhow::anyhow!("relay peer public key must be 32 bytes"))
        })?;
        let tx = self.db.begin_write()?;
        let writer = RelayPeerWriter::new(&tx);
        writer.delete_by_public_key(&peer_public_key)?;
        tx.commit()?;
        Ok(())
    }
}
