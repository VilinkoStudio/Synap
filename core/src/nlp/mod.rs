mod internal;

pub mod embedding;
pub mod metrics;
pub mod tag;
#[cfg(test)]
mod test;
pub mod traits;
pub mod types;

pub use embedding::{EmbeddingError, EmbeddingModel, LocalHashEmbedding, OpenAiEmbeddingModel};
pub use tag::NlpTagIndex;
pub use traits::{LearnableTextEncoder, TextEncoder};
pub use types::{NlpDocument, TagSuggestion};
