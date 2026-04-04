//! List command implementation.

use synap_core::SynapService;

use crate::{
    output,
    support::{fetch_all_notes_by_tag, fetch_all_recent},
};

pub fn execute(
    service: &SynapService,
    tag_filter: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let notes = if let Some(tag) = tag_filter {
        fetch_all_notes_by_tag(service, &tag)?
    } else {
        fetch_all_recent(service)?
    };

    output::format_note_list(&notes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_list_all() {
        let service = SynapService::open_memory().unwrap();
        let result = execute(&service, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_list_by_tag() {
        let service = SynapService::open_memory().unwrap();
        let result = execute(&service, Some("rust".to_string()));
        assert!(result.is_ok());
    }
}
