//! Void command implementation - logical deletion.

use colored::Colorize;
use synap_core::SynapService;

use crate::support::resolve_note_prefix;

pub fn execute(id_prefix: &str, service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    let note = resolve_note_prefix(service, id_prefix)?;
    service.delete_note(&note.id)?;

    println!(
        "{} 宣告死亡: TOMBSTONE ──[废弃]─> ({})",
        "[-]".red(),
        note.id[..8].red()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_success() {
        let service = SynapService::open_memory().unwrap();
        let note = service.create_note("Test".to_string(), vec![]).unwrap();
        let short_id = &note.id[..8];

        let result = execute(short_id, &service);
        assert!(result.is_ok());

        let notes = service.get_recent_note(None, Some(10)).unwrap();
        assert!(notes.is_empty());
    }
}
