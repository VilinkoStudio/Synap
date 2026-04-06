mod db;
mod text;

// Public API modules
pub mod envelope;
pub mod models;
pub mod nlp;
pub mod service;
pub mod sync;

pub mod dto;
pub mod error;
pub mod search;
pub mod version;
pub mod views;
/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use dto::{NoteDTO, TimelineNotesPageDTO, TimelineSessionDTO, TimelineSessionsPageDTO};
pub use error::{NoteError, ServiceError};
pub use service::{SynapService, TimelineDirection};
pub use version::{build_info, version_string, BuildInfo};
