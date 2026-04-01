use divan::Bencher;
use synap_core::dto::NoteDTO;

mod fixture;

use fixture::BenchFixture;

#[divan::bench(args = fixture::CONFIGS)]
fn bench_get_replies(bencher: Bencher, config: fixture::BenchConfig) {
    let fixture = BenchFixture::new(config);
    bencher.bench_local(|| {
        let parent_id = &fixture.parent_ids[0];
        fixture.service.get_replies(parent_id, None, 20).unwrap()
    });
}

#[divan::bench(args = fixture::CONFIGS)]
fn bench_get_origins(bencher: Bencher, config: fixture::BenchConfig) {
    let fixture = BenchFixture::new(config);
    bencher.bench_local(|| {
        let leaf_id = &fixture.deep_leaf_ids[0];
        fixture.service.get_origins(leaf_id).unwrap()
    });
}

#[divan::bench(args = fixture::CONFIGS)]
fn bench_get_previous_versions(bencher: Bencher, config: fixture::BenchConfig) {
    let fixture = BenchFixture::new(config);
    bencher.bench_local(|| {
        let note_id = &fixture.versioned_note_ids[0];
        fixture.service.get_previous_versions(note_id).unwrap()
    });
}

#[divan::bench(args = fixture::CONFIGS)]
fn bench_get_next_versions(bencher: Bencher, config: fixture::BenchConfig) {
    let fixture = BenchFixture::new(config);
    bencher.bench_local(|| {
        let note_id = &fixture.versioned_note_ids[0];
        fixture.service.get_next_versions(note_id).unwrap()
    });
}

#[divan::bench(args = fixture::CONFIGS)]
fn bench_get_other_versions(bencher: Bencher, config: fixture::BenchConfig) {
    let fixture = BenchFixture::new(config);
    bencher.bench_local(|| {
        let note_id = &fixture.versioned_note_ids[0];
        fixture.service.get_other_versions(note_id).unwrap()
    });
}

#[divan::bench(args = fixture::CONFIGS)]
fn bench_search_notes(bencher: Bencher, config: fixture::BenchConfig) {
    let fixture = BenchFixture::new(config);
    bencher.bench_local(|| fixture.service.search("content", 20).unwrap());
}

#[divan::bench(args = fixture::CONFIGS)]
fn bench_search_tags(bencher: Bencher, config: fixture::BenchConfig) {
    let fixture = BenchFixture::new(config);
    bencher.bench_local(|| fixture.service.search_tags("tag", 20).unwrap());
}

#[divan::bench(args = fixture::CONFIGS)]
fn bench_create_note(bencher: Bencher, config: fixture::BenchConfig) {
    bencher
        .with_inputs(|| BenchFixture::new(config))
        .bench_local_values(|fixture| {
            let tag = fixture.tag_names[0].clone();
            fixture
                .service
                .create_note("benchmark note content".to_string(), vec![tag])
                .unwrap()
        });
}

#[divan::bench(args = fixture::CONFIGS)]
fn bench_edit_note(bencher: Bencher, config: fixture::BenchConfig) {
    bencher
        .with_inputs(|| BenchFixture::new(config))
        .bench_local_values(|fixture| {
            let note_id = fixture.note_ids[0].clone();
            let tag = fixture.tag_names[0].clone();
            fixture
                .service
                .edit_note(&note_id, "edited content".to_string(), vec![tag])
                .unwrap()
        });
}

#[divan::bench(args = fixture::CONFIGS)]
fn bench_reply_note(bencher: Bencher, config: fixture::BenchConfig) {
    bencher
        .with_inputs(|| BenchFixture::new(config))
        .bench_local_values(|fixture| {
            let parent_id = fixture.parent_ids[0].clone();
            let tag = fixture.tag_names[0].clone();
            fixture
                .service
                .reply_note(&parent_id, "reply content".to_string(), vec![tag])
                .unwrap()
        });
}

fn main() {
    divan::main();
}
