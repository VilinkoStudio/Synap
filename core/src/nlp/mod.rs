mod index;
#[cfg(test)]
mod test;
mod text;
pub mod types;

pub use index::NlpTagIndex;
pub use types::{NlpDocument, TagSuggestion};
