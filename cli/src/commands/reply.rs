//! Reply command implementation - thought extension.

use colored::Colorize;
use synap_core::SynapService;

use crate::support::{extract_tags, resolve_note_prefix};

pub fn execute(
    target_id_prefix: &str,
    content: &str,
    service: &SynapService,
) -> Result<(), Box<dyn std::error::Error>> {
    let parent = resolve_note_prefix(service, target_id_prefix)?;
    let child = service.reply_note(&parent.id, content.to_string(), extract_tags(content))?;

    println!(
        "{} 思维延伸: ({}) ──[回复]─> ({})",
        "[+]".green(),
        child.id[..8].cyan(),
        parent.id[..8].dimmed()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tags() {
        let content = "This is a test #rust #programming with multiple tags";
        let tags = extract_tags(content);
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"programming".to_string()));
    }
}
