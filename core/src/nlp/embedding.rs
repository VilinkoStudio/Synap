use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use crate::nlp::traits::TextEncoder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_EMBEDDING_DIMENSION: usize = 384;
const DEFAULT_OPENAI_EMBEDDINGS_ENDPOINT: &str = "https://api.openai.com/v1/embeddings";

pub trait EmbeddingModel: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        texts.iter().map(|text| self.embed(text)).collect()
    }

    fn dimension(&self) -> usize;
}

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("embedding input is empty")]
    EmptyInput,

    #[error("embedding request failed: {0}")]
    Transport(String),

    #[error("embedding api returned status {status}: {body}")]
    HttpStatus { status: u16, body: String },

    #[error("failed to encode or decode embedding json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("embedding api response is missing data")]
    MissingData,

    #[error("embedding api returned invalid vector for item {index}")]
    InvalidVector { index: usize },
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
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(self.encode(text))
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiEmbeddingModel {
    api_key: String,
    model: String,
    endpoint: String,
    dimension: usize,
    timeout: Duration,
}

impl OpenAiEmbeddingModel {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>, dimension: usize) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            endpoint: DEFAULT_OPENAI_EMBEDDINGS_ENDPOINT.to_string(),
            dimension,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn post_embeddings(
        &self,
        input: EmbeddingInput,
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let body = serde_json::to_string(&OpenAiEmbeddingRequest {
            model: self.model.clone(),
            input,
            dimensions: (self.dimension > 0).then_some(self.dimension),
            encoding_format: "float",
        })?;

        let mut response = ureq::post(&self.endpoint)
            .config()
            .timeout_global(Some(self.timeout))
            .http_status_as_error(false)
            .build()
            .header("authorization", &format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .send(body.as_str())
            .map_err(map_ureq_error)?;

        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().map_err(map_ureq_error)?;
        if !(200..300).contains(&status) {
            return Err(EmbeddingError::HttpStatus { status, body: text });
        }

        let parsed: OpenAiEmbeddingResponse = serde_json::from_str(&text)?;
        if parsed.data.is_empty() {
            return Err(EmbeddingError::MissingData);
        }

        let mut vectors = parsed.data;
        vectors.sort_by_key(|item| item.index);
        vectors
            .into_iter()
            .map(|item| {
                if item.embedding.is_empty() {
                    Err(EmbeddingError::InvalidVector { index: item.index })
                } else {
                    Ok(item.embedding)
                }
            })
            .collect()
    }
}

impl EmbeddingModel for OpenAiEmbeddingModel {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }

        let mut vectors = self.post_embeddings(EmbeddingInput::Single(text.to_string()))?;
        vectors.pop().ok_or(EmbeddingError::MissingData)
    }

    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let inputs = texts
            .iter()
            .map(|text| text.trim())
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        if inputs.is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }

        self.post_embeddings(EmbeddingInput::Batch(inputs))
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

#[derive(Debug, Serialize)]
struct OpenAiEmbeddingRequest {
    model: String,
    input: EmbeddingInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
    encoding_format: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum EmbeddingInput {
    Single(String),
    Batch(Vec<String>),
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<OpenAiEmbeddingItem>,
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbeddingItem {
    embedding: Vec<f32>,
    index: usize,
}

fn map_ureq_error(err: ureq::Error) -> EmbeddingError {
    match err {
        ureq::Error::StatusCode(status) => EmbeddingError::HttpStatus {
            status,
            body: String::new(),
        },
        other => EmbeddingError::Transport(other.to_string()),
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
        let vector = embedder.embed("hello world").unwrap();
        assert_eq!(vector.len(), DEFAULT_EMBEDDING_DIMENSION);
    }

    #[test]
    fn test_embedding_custom_dimension() {
        let embedder = LocalHashEmbedding::new(128);
        let vector = embedder.embed("hello world").unwrap();
        assert_eq!(vector.len(), 128);
    }

    #[test]
    fn test_embedding_deterministic() {
        let embedder = LocalHashEmbedding::new(384);
        let v1 = embedder.embed("hello world").unwrap();
        let v2 = embedder.embed("hello world").unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_embedding_different_texts() {
        let embedder = LocalHashEmbedding::new(384);
        let v1 = embedder.embed("hello world").unwrap();
        let v2 = embedder.embed("different text").unwrap();
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
        let vector = embedder.embed("test").unwrap();
        let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_openai_response_deserialize() {
        let json = r#"{
            "data": [
                { "index": 1, "embedding": [3.0, 4.0] },
                { "index": 0, "embedding": [1.0, 2.0] }
            ]
        }"#;

        let parsed: OpenAiEmbeddingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.data.len(), 2);
        assert_eq!(parsed.data[0].index, 1);
    }
}
