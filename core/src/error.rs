use std::array::TryFromSliceError;

use redb::Error as DbError;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum NoteError {
    // 透传 redb 错误，自动实现 From<redb::Error>
    #[error("database operation failed")]
    Db(#[from] DbError),

    // 自定义业务错误，带上下文数据
    #[error("note not found: {id}")]
    IdNotFound { id: Uuid },

    #[error("note not found: {:?}", id)]
    ShortIdNotFound { id: [u8; 8] },
    // 简单字符串错误（适合输入验证）
    #[error("invalid title: {0}")]
    InvalidTitle(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Database error: {0}")]
    Db(#[from] redb::Error),

    #[error("Transaction error: {0}")]
    TransactionErr(#[from] redb::TransactionError),

    #[error("Commit error: {0}")]
    CommitErr(#[from] redb::CommitError),

    #[error("Note not found: {0}")]
    NotFound(String),

    #[error("Invalid ID format")]
    InvalidId,

    #[error("Tempfile error")]
    TempfileIO(()),

    #[error("Uuid error: {0}")]
    UuidErr(#[from] uuid::Error),

    #[error("note error: {0}")]
    NoteErr(#[from] crate::error::NoteError),

    #[error("ID length error")]
    SliceErr(#[from] TryFromSliceError),

    #[error("error")]
    Err(()),

    #[error(transparent)]
    Other(#[from] anyhow::Error),

    #[error("share protocol error: {0}")]
    ShareProtocol(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
