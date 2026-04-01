use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::BuildHasherDefault,
};

pub(super) type StableState = BuildHasherDefault<DefaultHasher>;
pub(super) type StableMap<K, V> = HashMap<K, V, StableState>;
pub(super) type StableSet<T> = HashSet<T, StableState>;
pub(super) type GramId = u32;
pub(super) type SparseVector = Vec<(GramId, f32)>;
pub(super) type GramWeights = StableMap<GramId, f32>;

pub(super) fn stable_state() -> StableState {
    StableState::default()
}

pub(super) fn stable_map<K, V>() -> StableMap<K, V> {
    HashMap::with_hasher(stable_state())
}

pub(super) fn stable_set<T>() -> StableSet<T> {
    HashSet::with_hasher(stable_state())
}

pub(super) fn gram_weights() -> GramWeights {
    stable_map()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NlpDocument {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub active: bool,
}

impl NlpDocument {
    pub fn new(
        id: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            tags,
            active: true,
        }
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagSuggestion {
    pub tag: String,
    pub score: f32,
}
