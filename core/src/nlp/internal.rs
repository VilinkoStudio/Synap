use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::BuildHasherDefault,
};

pub(crate) type StableState = BuildHasherDefault<DefaultHasher>;
pub(crate) type StableMap<K, V> = HashMap<K, V, StableState>;
pub(crate) type StableSet<T> = HashSet<T, StableState>;
pub(crate) type GramId = u32;
pub(crate) type SparseVector = Vec<(GramId, f32)>;
pub(crate) type GramWeights = StableMap<GramId, f32>;

pub(crate) fn stable_state() -> StableState {
    StableState::default()
}

pub(crate) fn stable_map<K, V>() -> StableMap<K, V> {
    HashMap::with_hasher(stable_state())
}

pub(crate) fn stable_set<T>() -> StableSet<T> {
    HashSet::with_hasher(stable_state())
}

pub(crate) fn gram_weights() -> GramWeights {
    stable_map()
}
