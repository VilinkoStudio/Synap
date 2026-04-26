//! FFI-compatible error types for Synap.

use synap_core::error::{NoteError, ServiceError};
use uniffi::UnexpectedUniFFICallbackError;

/// FFI-compatible error type.
#[derive(Debug, Clone, thiserror::Error)]
pub enum FfiError {
    #[error("Database error")]
    Database,

    #[error("Note not found")]
    NotFound,

    #[error("Invalid id")]
    InvalidId,

    #[error("I/O error")]
    Io,

    #[error("Other error")]
    Other,
}

impl From<ServiceError> for FfiError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::Db(_) | ServiceError::TransactionErr(_) | ServiceError::CommitErr(_) => {
                FfiError::Database
            }
            ServiceError::NotFound(_) => FfiError::NotFound,
            ServiceError::InvalidId | ServiceError::UuidErr(_) | ServiceError::SliceErr(_) => {
                FfiError::InvalidId
            }
            ServiceError::TempfileIO(()) | ServiceError::Io(_) => FfiError::Io,
            ServiceError::NoteErr(err) => err.into(),
            ServiceError::Err(())
            | ServiceError::Other(_)
            | ServiceError::ShareProtocol(_)
            | ServiceError::Embedding(_) => {
                FfiError::Other
            }
        }
    }
}

impl From<NoteError> for FfiError {
    fn from(err: NoteError) -> Self {
        match err {
            NoteError::Db(_) => FfiError::Database,
            NoteError::IdNotFound { .. } | NoteError::ShortIdNotFound { .. } => FfiError::NotFound,
            NoteError::InvalidTitle(_) => FfiError::Other,
        }
    }
}

impl From<std::io::Error> for FfiError {
    fn from(_err: std::io::Error) -> Self {
        FfiError::Io
    }
}

impl From<UnexpectedUniFFICallbackError> for FfiError {
    fn from(_err: UnexpectedUniFFICallbackError) -> Self {
        FfiError::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_from_service_error() {
        let err: FfiError = ServiceError::NotFound("missing".to_string()).into();
        assert!(matches!(err, FfiError::NotFound));
    }

    #[test]
    fn test_error_from_service_io_error() {
        let err: FfiError = ServiceError::Io(std::io::Error::other("disk failed")).into();
        assert!(matches!(err, FfiError::Io));
    }

    #[test]
    fn test_error_from_service_share_protocol_error() {
        let err: FfiError = ServiceError::ShareProtocol("bad payload".to_string()).into();
        assert!(matches!(err, FfiError::Other));
    }

    #[test]
    fn test_error_from_note_error() {
        let err: FfiError = NoteError::InvalidTitle("bad".to_string()).into();
        assert!(matches!(err, FfiError::Other));
    }
}
