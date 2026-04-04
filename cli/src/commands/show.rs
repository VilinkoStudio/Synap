//! Show command implementation with prefix support.

use synap_core::SynapService;

use crate::{
    output,
    support::{fetch_all_replies, resolve_note_prefix},
};

pub fn execute(id_prefix: &str, service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    let note = resolve_note_prefix(service, id_prefix)?;
    output::format_note_detail(&note);

    let origins = service.get_origins(&note.id)?;
    if !origins.is_empty() {
        println!("来源:");
        output::format_note_list(&origins);
    }

    let replies = fetch_all_replies(service, &note.id)?;
    if !replies.is_empty() {
        println!("回复:");
        output::format_note_list(&replies);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_with_prefix() {
        let service = SynapService::open_memory().unwrap();
        let note = service.create_note("Test".to_string(), vec![]).unwrap();
        let short_id = &note.id[..8];

        let result = execute(short_id, &service);
        assert!(result.is_ok());
    }
}
