//! Stats command implementation.

use std::collections::HashMap;

use synap_core::SynapService;

use crate::{
    output::{self, CliStats},
    support::{fetch_all_deleted, fetch_all_recent},
};

pub fn execute(service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    let live_notes = fetch_all_recent(service)?;
    let deleted_notes = fetch_all_deleted(service)?;

    let mut tag_counts = HashMap::<String, usize>::new();
    for note in &live_notes {
        for tag in &note.tags {
            *tag_counts.entry(tag.clone()).or_default() += 1;
        }
    }

    let mut top_tags: Vec<_> = tag_counts.into_iter().collect();
    top_tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top_tags.truncate(10);

    let stats = CliStats {
        total_live_notes: live_notes.len(),
        total_deleted_notes: deleted_notes.len(),
        total_tags: service.get_all_tags()?.len(),
        top_tags,
    };

    output::format_stats(&stats);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute() {
        let service = SynapService::open_memory().unwrap();
        let result = execute(&service);
        assert!(result.is_ok());
    }
}
