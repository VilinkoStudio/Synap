//! Search command implementation with #tag support.

use synap_core::SynapService;

use crate::output;

pub fn execute(query: &str, service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    if query.trim().is_empty() {
        return Err("搜索内容不能为空".into());
    }

    if let Some(tag) = query.strip_prefix('#') {
        return execute_tag(tag, service);
    }

    let results = service.search(query, 50)?;
    output::format_note_list(&results);
    Ok(())
}

pub fn execute_tag(tag: &str, service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    let notes = service.get_notes_by_tag(tag, None, Some(50))?;
    output::format_note_list(&notes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_with_results() {
        let service = SynapService::open_memory().unwrap();
        service
            .create_note("Test note about rust".to_string(), vec![])
            .unwrap();
        let result = execute("rust", &service);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_empty_query() {
        let service = SynapService::open_memory().unwrap();
        let result = execute("", &service);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_tag_search() {
        let service = SynapService::open_memory().unwrap();
        service
            .create_note("Test note".to_string(), vec!["rust".to_string()])
            .unwrap();

        let result = execute("#rust", &service);
        assert!(result.is_ok());
    }
}
