use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::nlp::traits::TextEncoder;

const DEFAULT_EMBEDDING_DIMENSION: usize = 384;

pub trait EmbeddingModel: TextEncoder<Vec<f32>> {
    fn embed(&self, text: &str) -> Vec<f32> {
        self.encode(text)
    }

    fn dimension(&self) -> usize;
}

pub struct LocalHashEmbedding {
    dimension: usize,
}

impl LocalHashEmbedding {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension: if dimension > 0 {
                dimension
            } else {
                DEFAULT_EMBEDDING_DIMENSION
            },
        }
    }
}

impl TextEncoder<Vec<f32>> for LocalHashEmbedding {
    fn encode(&self, text: &str) -> Vec<f32> {
        let tokens = tokenize_for_embedding(text);
        let tokens = if tokens.is_empty() {
            vec!["__empty__".to_string()]
        } else {
            tokens
        };

        let mut vector = vec![0.0_f64; self.dimension];

        for token in &tokens {
            let hash_a = hash_token(format!("{}\x00{}", "synap", token));
            let hash_b = hash_token(format!("{}\x00{}", token, "synap"));

            let index_a = (hash_a % self.dimension as u64) as usize;
            let index_b = (hash_b % self.dimension as u64) as usize;

            vector[index_a] += signed_weight(hash_a, 1.0);
            vector[index_b] += signed_weight(hash_b, 0.5);
        }

        let norm: f64 = vector.iter().map(|v| v * v).sum::<f64>().sqrt();
        let norm = if norm == 0.0 { 1.0 } else { norm };

        vector.iter().map(|v| (v / norm) as f32).collect()
    }
}

impl EmbeddingModel for LocalHashEmbedding {
    fn dimension(&self) -> usize {
        self.dimension
    }
}

fn tokenize_for_embedding(text: &str) -> Vec<String> {
    let normalized = text.trim().to_lowercase();
    if normalized.is_empty() {
        return vec![];
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut han_runes: Vec<char> = Vec::new();

    for r in normalized.chars() {
        if r.is_alphanumeric() {
            if is_han(r) {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                han_runes.push(r);
                tokens.push(r.to_string());
            } else {
                current.push(r);
            }
        } else if !current.is_empty() {
            tokens.push(current.clone());
            current.clear();
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    for i in 0..han_runes.len().saturating_sub(1) {
        tokens.push(han_runes[i].to_string() + &han_runes[i + 1].to_string());
    }

    if tokens.is_empty() {
        let runes: Vec<char> = normalized.chars().collect();
        for i in 0..runes.len().saturating_sub(1) {
            tokens.push(runes[i].to_string() + &runes[i + 1].to_string());
        }
    }

    tokens
}

fn is_han(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c)
}

fn hash_token(value: String) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn signed_weight(hash: u64, weight: f64) -> f64 {
    if hash & 1 == 0 {
        weight
    } else {
        -weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_dimensions() {
        let embedder = LocalHashEmbedding::new(0);
        let vector = embedder.embed("hello world");
        assert_eq!(vector.len(), DEFAULT_EMBEDDING_DIMENSION);
    }

    #[test]
    fn test_embedding_custom_dimension() {
        let embedder = LocalHashEmbedding::new(128);
        let vector = embedder.embed("hello world");
        assert_eq!(vector.len(), 128);
    }

    #[test]
    fn test_embedding_deterministic() {
        let embedder = LocalHashEmbedding::new(384);
        let v1 = embedder.embed("hello world");
        let v2 = embedder.embed("hello world");
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_embedding_different_texts() {
        let embedder = LocalHashEmbedding::new(384);
        let v1 = embedder.embed("hello world");
        let v2 = embedder.embed("different text");
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_tokenize_english() {
        let tokens = tokenize_for_embedding("Hello World");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
    }

    #[test]
    fn test_tokenize_chinese() {
        let tokens = tokenize_for_embedding("你好世界");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_embedding_normalized() {
        let embedder = LocalHashEmbedding::new(384);
        let vector = embedder.embed("test");
        let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }
}
