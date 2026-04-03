pub trait TextEncoder<V>: Send + Sync {
    fn encode(&self, text: &str) -> V;
}

pub trait LearnableTextEncoder<V>: TextEncoder<V> {
    fn encode_with_updates(&mut self, text: &str) -> V;
}
