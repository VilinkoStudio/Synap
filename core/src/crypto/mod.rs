mod channel;
mod fingerprint;
mod identity;
mod signing;
mod trust;

pub use channel::{
    AuthenticatedPeer, CryptoChannel, CryptoChannelError, CryptoChannelMode, CryptoChannelOptions,
    PeerIdentity,
};
pub use fingerprint::generate_kaomoji_fingerprint;
pub use identity::{
    ensure_local_identity, local_identity_key_id, local_identity_private_key,
    local_identity_public_key, local_identity_secret_id,
};
pub use signing::{
    ensure_local_signing_identity, local_signing_key_id, local_signing_public_key,
    local_signing_secret_id, sign_with_local_identity, verify_signed_bytes,
};
pub use trust::{
    delete_trusted_public_key, get_known_public_key, get_known_public_key_by_bytes,
    get_trusted_public_key, get_trusted_public_key_by_bytes, import_trusted_public_key,
    is_trusted_public_key, list_known_public_keys, list_trusted_public_keys,
    public_key_fingerprint, remember_untrusted_public_key, update_trusted_public_key_note,
    update_trusted_public_key_status, TrustedPublicKeyRecord,
};
