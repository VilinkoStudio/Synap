use redb::ReadableDatabase;

use crate::{crypto, error::ServiceError, models::crypto::CryptoReader};

use super::SynapService;

const MDNS_SIGNING_CONTEXT: &[u8] = b"synap.mdns.discovery.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdnsDiscoveryError {
    InvalidPublicKey,
    InvalidSignature,
    InvalidHex,
}

impl std::fmt::Display for MdnsDiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPublicKey => write!(f, "invalid ed25519 public key"),
            Self::InvalidSignature => write!(f, "invalid ed25519 signature"),
            Self::InvalidHex => write!(f, "invalid hex encoding"),
        }
    }
}

impl std::error::Error for MdnsDiscoveryError {}

impl From<MdnsDiscoveryError> for ServiceError {
    fn from(err: MdnsDiscoveryError) -> Self {
        ServiceError::Other(anyhow::anyhow!(err))
    }
}

/// Construct the signing payload for mDNS discovery.
///
/// Format: `MDNS_SIGNING_CONTEXT || signing_public_key[32]`
fn mdns_signing_payload(signing_public_key: &[u8; 32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(MDNS_SIGNING_CONTEXT.len() + 32);
    payload.extend_from_slice(MDNS_SIGNING_CONTEXT);
    payload.extend_from_slice(signing_public_key);
    payload
}

/// Parse and verify mDNS discovery TXT record fields.
///
/// Returns the verified signing public key on success.
pub fn verify_mdns_discovery_txt(
    key_hex: &str,
    sig_hex: &str,
) -> Result<[u8; 32], MdnsDiscoveryError> {
    let key_bytes = hex_decode_32(key_hex).map_err(|_| MdnsDiscoveryError::InvalidHex)?;
    let sig_bytes = hex_decode_64(sig_hex).map_err(|_| MdnsDiscoveryError::InvalidHex)?;

    verify_mdns_discovery_signature(&key_bytes, &sig_bytes)?;
    Ok(key_bytes)
}

/// Verify an mDNS discovery signature.
pub fn verify_mdns_discovery_signature(
    signing_public_key: &[u8; 32],
    signature: &[u8; 64],
) -> Result<(), MdnsDiscoveryError> {
    let payload = mdns_signing_payload(signing_public_key);
    if !crypto::verify_signed_bytes(*signing_public_key, &payload, *signature) {
        return Err(MdnsDiscoveryError::InvalidSignature);
    }

    Ok(())
}

impl SynapService {
    /// Sign an mDNS discovery broadcast.
    ///
    /// Returns `(signing_public_key, signature)`.
    pub fn sign_mdns_discovery(&self) -> Result<([u8; 32], [u8; 64]), ServiceError> {
        let tx = self.db.begin_read()?;
        let reader = CryptoReader::new(&tx)?;
        let public_key = crypto::local_signing_public_key(&reader)?
            .ok_or_else(|| ServiceError::Other(anyhow::anyhow!("local signing identity is missing")))?;

        let payload = mdns_signing_payload(&public_key);
        let signature = crypto::sign_with_local_identity(&reader, &payload)?
            .ok_or_else(|| ServiceError::Other(anyhow::anyhow!("local signing identity is missing")))?;

        Ok((public_key, signature))
    }

    /// Verify an mDNS discovery broadcast from TXT record hex fields.
    ///
    /// Returns the verified signing public key on success.
    pub fn verify_mdns_discovery(
        key_hex: &str,
        sig_hex: &str,
    ) -> Result<[u8; 32], ServiceError> {
        verify_mdns_discovery_txt(key_hex, sig_hex).map_err(Into::into)
    }
}

fn hex_decode_32(hex: &str) -> Result<[u8; 32], ()> {
    let bytes = hex::decode(hex).map_err(|_| ())?;
    bytes.try_into().map_err(|_| ())
}

fn hex_decode_64(hex: &str) -> Result<[u8; 64], ()> {
    let bytes = hex::decode(hex).map_err(|_| ())?;
    bytes.try_into().map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_service() -> SynapService {
        SynapService::open_memory().unwrap()
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let service = setup_service();
        let (pub_key, sig) = service.sign_mdns_discovery().unwrap();

        let key_hex = hex::encode(pub_key);
        let sig_hex = hex::encode(sig);

        let verified = verify_mdns_discovery_txt(&key_hex, &sig_hex).unwrap();
        assert_eq!(verified, pub_key);
    }

    #[test]
    fn verify_rejects_wrong_public_key() {
        let service = setup_service();
        let (_, sig) = service.sign_mdns_discovery().unwrap();

        let wrong_key = [99u8; 32];
        let wrong_hex = hex::encode(wrong_key);
        let sig_hex = hex::encode(sig);

        assert!(matches!(
            verify_mdns_discovery_txt(&wrong_hex, &sig_hex),
            Err(MdnsDiscoveryError::InvalidSignature)
        ));
    }

    #[test]
    fn verify_rejects_bad_hex() {
        assert!(matches!(
            verify_mdns_discovery_txt("zzzz", "aaaa"),
            Err(MdnsDiscoveryError::InvalidHex)
        ));
    }

    #[test]
    fn verify_rejects_wrong_signature() {
        let service = setup_service();
        let (pub_key, _) = service.sign_mdns_discovery().unwrap();

        let wrong_sig = [42u8; 64];
        let key_hex = hex::encode(pub_key);
        let sig_hex = hex::encode(wrong_sig);

        assert!(matches!(
            verify_mdns_discovery_txt(&key_hex, &sig_hex),
            Err(MdnsDiscoveryError::InvalidSignature)
        ));
    }
}
