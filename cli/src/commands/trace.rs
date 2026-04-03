//! Trace command implementation - graph topology rendering.

use synap_core::{NoteDTO, SynapService};

use crate::{
    output,
    support::{build_reply_tree, fetch_all_recent, resolve_note_prefix},
};

pub fn execute(
    id_prefix: Option<&str>,
    service: &SynapService,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(id_prefix) = id_prefix {
        let note = resolve_note_prefix(service, id_prefix)?;
        let graph = build_reply_tree(service, note)?;
        render_graph_ascii(graph)?;
    } else {
        execute_recent(service)?;
    }

    Ok(())
}

pub fn execute_recent(service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    let recent = service.get_recent_note(None, Some(20))?;
    output::format_note_list(&recent);
    Ok(())
}

pub fn execute_stream(service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    let all = fetch_all_recent(service)?;
    output::format_note_list(&all);
    Ok(())
}

pub fn render_graph_ascii(graph: Vec<(NoteDTO, usize)>) -> Result<(), Box<dyn std::error::Error>> {
    output::format_graph(&graph);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_with_no_notes() {
        let service = SynapService::open_memory().unwrap();
        let result = execute(None, &service);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_with_notes() {
        let service = SynapService::open_memory().unwrap();
        let note = service.create_note("Test".to_string(), vec![]).unwrap();
        let short_id = &note.id[..8];

        let result = execute(Some(short_id), &service);
        assert!(result.is_ok());
    }
}
