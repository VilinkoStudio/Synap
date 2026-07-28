pub trait TextEncoder<V>: Send + Sync {
    fn encode(&self, text: &str) -> V;
}
