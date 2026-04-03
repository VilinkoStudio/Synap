//! Capture command implementation - zero-friction thought capture.

use colored::Colorize;
use synap_core::SynapService;

use crate::support::extract_tags;

pub fn execute(content: &str, service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    let note = service.create_note(content.to_string(), extract_tags(content))?;

    println!("{} 笔记已创建 ({})", "[+]".green(), note.id[..8].cyan());
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

    #[test]
    fn test_extract_no_tags() {
        let content = "This is a test without tags";
        let tags = extract_tags(content);
        assert_eq!(tags.len(), 0);
    }

    #[test]
    fn test_extract_empty_tag() {
        let content = "This is a test # with empty tag";
        let tags = extract_tags(content);
        assert!(tags.is_empty());
    }
}
