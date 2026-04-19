use uuid::Uuid;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::models::crypto::{
    CryptoReader, CryptoWriter, KeyMetadataRecord, KeyPurpose, KeyStatus, KeyVisibility,
    SensitiveKeyRecord,
};

// 为本地数据库身份预留固定主键。
// 这样升级、重启、重新打开数据库时，都可以用稳定 ID 直接定位身份记录，
// 不需要额外扫表或再建一层索引。
const LOCAL_IDENTITY_KEY_ID: Uuid = Uuid::from_u128(0x0f4db6f5_f369_4f11_a6be_36a23a3db001);
const LOCAL_IDENTITY_SECRET_ID: Uuid = Uuid::from_u128(0x0f4db6f5_f369_4f11_a6be_36a23a3db002);

pub fn local_identity_key_id() -> Uuid {
    LOCAL_IDENTITY_KEY_ID
}

pub fn local_identity_secret_id() -> Uuid {
    LOCAL_IDENTITY_SECRET_ID
}

/// 确保当前数据库存在一组可用于通信身份标识的本地 X25519 密钥对。
///
/// 这是一个幂等初始化入口：
/// - 记录已存在时，只做一致性校验并直接返回
/// - 记录不存在时，自动生成并落库
pub fn ensure_local_identity(writer: &CryptoWriter<'_>) -> Result<KeyMetadataRecord, redb::Error> {
    let metadata = writer.get_metadata(LOCAL_IDENTITY_KEY_ID)?;
    let secret = writer.get_sensitive(LOCAL_IDENTITY_SECRET_ID)?;

    if let (Some(metadata), Some(secret)) = (metadata, secret) {
        validate_local_identity(&metadata, &secret)?;
        return Ok(metadata);
    }

    let secret = StaticSecret::random();
    let public = PublicKey::from(&secret);

    let metadata = KeyMetadataRecord {
        id: LOCAL_IDENTITY_KEY_ID,
        name: Some("local database identity".into()),
        note: None,
        kind: "keypair".into(),
        algorithm: "x25519".into(),
        purpose: KeyPurpose::Identity,
        visibility: KeyVisibility::Secret,
        status: KeyStatus::Active,
        fingerprint: None,
        secret_ref: Some(LOCAL_IDENTITY_SECRET_ID),
        kek_ref: None,
        public_key: Some(public.as_bytes().to_vec()),
    };

    let sensitive = SensitiveKeyRecord {
        id: LOCAL_IDENTITY_SECRET_ID,
        owner_key_id: LOCAL_IDENTITY_KEY_ID,
        encoding: "raw32".into(),
        private_key: secret.to_bytes().to_vec(),
    };

    writer.put_metadata(&metadata)?;
    writer.put_sensitive(&sensitive)?;

    Ok(metadata)
}

/// 读取当前数据库的身份公钥。
///
/// 这里故意只暴露公钥，调用方如果只是做“对外宣告身份”或“发起密钥协商”，
/// 不需要知道私钥的存储细节。
pub fn local_identity_public_key(
    reader: &CryptoReader<'_>,
) -> Result<Option<[u8; 32]>, redb::Error> {
    let Some(metadata) = reader.get_metadata(&LOCAL_IDENTITY_KEY_ID)? else {
        return Ok(None);
    };
    let Some(public_key) = metadata.public_key else {
        return Err(invalid_crypto_record(
            "local identity metadata missing public key",
        ));
    };
    let bytes: [u8; 32] = public_key
        .as_slice()
        .try_into()
        .map_err(|_| invalid_crypto_record("local identity public key must be 32 bytes"))?;
    Ok(Some(bytes))
}

/// 读取当前数据库的身份私钥。
///
/// 这里只负责把 model 层中保存的原始字节取出并校验长度，
/// 不在这里引入更高层的加解密协议。
pub fn local_identity_private_key(
    reader: &CryptoReader<'_>,
) -> Result<Option<StaticSecret>, redb::Error> {
    let Some(secret) = reader.get_sensitive(&LOCAL_IDENTITY_SECRET_ID)? else {
        return Ok(None);
    };
    if secret.encoding != "raw32" {
        return Err(invalid_crypto_record(
            "local identity secret encoding must be raw32",
        ));
    }
    let bytes: [u8; 32] = secret
        .private_key
        .as_slice()
        .try_into()
        .map_err(|_| invalid_crypto_record("local identity secret must be 32 bytes"))?;
    Ok(Some(StaticSecret::from(bytes)))
}

fn validate_local_identity(
    metadata: &KeyMetadataRecord,
    secret: &SensitiveKeyRecord,
) -> Result<(), redb::Error> {
    // 启动时如果发现保留身份记录已经存在，这里负责做最小但关键的一致性校验，
    // 防止“metadata 和 secret 各自存在，但彼此不匹配”这类半损坏状态悄悄混过去。
    if metadata.algorithm != "x25519" {
        return Err(invalid_crypto_record(
            "local identity algorithm must be x25519",
        ));
    }
    if metadata.secret_ref != Some(LOCAL_IDENTITY_SECRET_ID) {
        return Err(invalid_crypto_record(
            "local identity metadata must point to reserved secret record",
        ));
    }

    let public_key = metadata
        .public_key
        .as_ref()
        .ok_or_else(|| invalid_crypto_record("local identity metadata missing public key"))?;
    let public_key: [u8; 32] = public_key
        .as_slice()
        .try_into()
        .map_err(|_| invalid_crypto_record("local identity public key must be 32 bytes"))?;

    if secret.owner_key_id != LOCAL_IDENTITY_KEY_ID {
        return Err(invalid_crypto_record(
            "local identity secret owner must point to reserved identity record",
        ));
    }
    if secret.encoding != "raw32" {
        return Err(invalid_crypto_record(
            "local identity secret encoding must be raw32",
        ));
    }
    let private_key: [u8; 32] = secret
        .private_key
        .as_slice()
        .try_into()
        .map_err(|_| invalid_crypto_record("local identity secret must be 32 bytes"))?;
    let expected_public = PublicKey::from(&StaticSecret::from(private_key));

    if expected_public.as_bytes() != &public_key {
        return Err(invalid_crypto_record(
            "local identity secret/public key material does not match",
        ));
    }

    Ok(())
}

fn invalid_crypto_record(message: &'static str) -> redb::Error {
    redb::Error::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message,
    ))
}

#[cfg(test)]
mod tests {
    use redb::{Database, ReadableDatabase};
    use tempfile::NamedTempFile;

    use super::*;

    fn temp_db() -> Database {
        let f = NamedTempFile::new().unwrap();
        Database::create(f.path()).unwrap()
    }

    #[test]
    fn test_ensure_local_identity_is_idempotent() {
        let db = temp_db();

        let wtx = db.begin_write().unwrap();
        CryptoWriter::init_schema(&wtx).unwrap();
        let writer = CryptoWriter::new(&wtx);
        let created = ensure_local_identity(&writer).unwrap();
        let created_public = created.public_key.clone().unwrap();
        wtx.commit().unwrap();

        let wtx = db.begin_write().unwrap();
        let writer = CryptoWriter::new(&wtx);
        let ensured = ensure_local_identity(&writer).unwrap();
        wtx.commit().unwrap();

        assert_eq!(ensured.id, LOCAL_IDENTITY_KEY_ID);
        assert_eq!(ensured.public_key, Some(created_public));

        let rtx = db.begin_read().unwrap();
        let reader = CryptoReader::new(&rtx).unwrap();
        let public = local_identity_public_key(&reader).unwrap().unwrap();
        assert_eq!(public.to_vec(), ensured.public_key.unwrap());
    }
}
