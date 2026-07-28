pub mod embedding;
pub mod metrics;
pub mod tag;
pub mod traits;
pub mod types;

pub use embedding::{EmbeddingError, EmbeddingModel, LocalHashEmbedding, OpenAiEmbeddingModel};
pub use traits::TextEncoder;
pub use types::TagSuggestion;
