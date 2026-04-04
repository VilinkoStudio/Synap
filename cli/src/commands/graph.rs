//! Graph command implementation.

use synap_core::SynapService;

use crate::{
    output,
    support::{build_reply_tree, latest_note, resolve_note_prefix},
};

pub fn execute(
    id_prefix: Option<&str>,
    service: &SynapService,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = if let Some(id_prefix) = id_prefix {
        resolve_note_prefix(service, id_prefix)?
    } else {
        latest_note(service)?
    };

    let graph = build_reply_tree(service, root)?;
    output::format_graph(&graph);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_graph_empty() {
        let service = SynapService::open_memory().unwrap();
        assert!(execute(None, &service).is_err());
    }
}
