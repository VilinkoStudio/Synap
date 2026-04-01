use std::fmt;

use synap_core::service::SynapService;

pub const SMALL: BenchConfig = BenchConfig {
    name: "small",
    note_count: 100,
    tag_count: 20,
    chain_depth: 3,
    version_ratio: 0.2,
};

pub const MEDIUM: BenchConfig = BenchConfig {
    name: "medium",
    note_count: 1_000,
    tag_count: 100,
    chain_depth: 5,
    version_ratio: 0.15,
};

pub const LARGE: BenchConfig = BenchConfig {
    name: "large",
    note_count: 3_000,
    tag_count: 200,
    chain_depth: 8,
    version_ratio: 0.1,
};

pub const CONFIGS: [BenchConfig; 3] = [SMALL, MEDIUM, LARGE];

#[derive(Clone, Copy, Debug)]
pub struct BenchConfig {
    pub name: &'static str,
    pub note_count: usize,
    pub tag_count: usize,
    pub chain_depth: usize,
    pub version_ratio: f32,
}

impl fmt::Display for BenchConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

pub struct BenchFixture {
    pub service: SynapService,
    pub note_ids: Vec<String>,
    pub parent_ids: Vec<String>,
    pub deep_leaf_ids: Vec<String>,
    pub versioned_note_ids: Vec<String>,
    pub tag_names: Vec<String>,
}

impl BenchFixture {
    pub fn new(config: BenchConfig) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("bench.redb");

        let service = SynapService::new(Some(db_path.to_string_lossy().into_owned())).unwrap();

        let mut fixture = Self {
            service,
            note_ids: Vec::new(),
            parent_ids: Vec::new(),
            deep_leaf_ids: Vec::new(),
            versioned_note_ids: Vec::new(),
            tag_names: Vec::new(),
        };

        fixture.generate_data(config);

        std::mem::forget(dir);

        fixture
    }

    fn generate_data(&mut self, config: BenchConfig) {
        let tag_names: Vec<String> = (0..config.tag_count).map(|i| format!("tag_{i}")).collect();

        for tag in &tag_names {
            let _ = self
                .service
                .create_note(format!("note with {tag}"), vec![tag.clone()]);
        }
        self.tag_names = tag_names;

        let root_count = config.note_count / 3;
        let mut all_note_ids: Vec<String> = Vec::new();

        for _ in 0..root_count {
            let tag = self.random_tag();
            let note = self
                .service
                .create_note("root note content".to_string(), vec![tag])
                .unwrap();
            all_note_ids.push(note.id.clone());
        }

        let reply_count = config.note_count * 2 / 3;
        let mut current_parents: Vec<String> = all_note_ids.clone();

        for i in 0..reply_count {
            let parent_idx = i % current_parents.len();
            let parent_id = current_parents[parent_idx].clone();

            let tag = self.random_tag();
            let child = self
                .service
                .reply_note(&parent_id, "reply content".to_string(), vec![tag])
                .unwrap();

            let child_id = child.id.clone();
            all_note_ids.push(child_id.clone());

            if i % 3 == 0 {
                self.parent_ids.push(parent_id);
            }

            if i % config.chain_depth.max(1) == 0 {
                current_parents.push(child_id.clone());
            }
        }

        let depth_leaf_count = (config.note_count / 20).min(20);
        if !current_parents.is_empty() {
            for i in 0..depth_leaf_count {
                let mut parent = current_parents[i % current_parents.len()].clone();

                for depth in 0..config.chain_depth.max(1) {
                    let tag = self.random_tag();
                    let child = self
                        .service
                        .reply_note(&parent, format!("deep leaf content {i}-{depth}"), vec![tag])
                        .unwrap();
                    parent = child.id.clone();
                }

                self.deep_leaf_ids.push(parent);
            }
        }

        let versioned_count = ((config.note_count as f32) * config.version_ratio) as usize;
        for note_id in all_note_ids
            .iter()
            .take(versioned_count.min(all_note_ids.len()))
        {
            let edited_v1 = self
                .service
                .edit_note(
                    note_id,
                    "edited version 1".to_string(),
                    vec![self.random_tag()],
                )
                .unwrap();
            let edited_v2 = self
                .service
                .edit_note(
                    &edited_v1.id,
                    "edited version 2".to_string(),
                    vec![self.random_tag()],
                )
                .unwrap();
            self.versioned_note_ids.push(edited_v2.id);
        }

        self.note_ids = all_note_ids;
    }

    fn random_tag(&self) -> String {
        let idx = pseudo_rand() % self.tag_names.len();
        self.tag_names[idx].clone()
    }
}

fn pseudo_rand() -> usize {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    hasher.write_u8(1);
    hasher.finish() as usize
}
