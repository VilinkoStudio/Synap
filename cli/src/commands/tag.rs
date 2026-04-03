//! Tag command implementation.

use synap_core::SynapService;

use crate::{
    output,
    support::{add_tag_to_note, remove_tag_from_note, resolve_note_prefix},
};

pub fn execute(
    id_prefix: &str,
    tag: &str,
    remove: bool,
    service: &SynapService,
) -> Result<(), Box<dyn std::error::Error>> {
    if tag.trim().is_empty() {
        return Err("标签名称不能为空".into());
    }

    let note = resolve_note_prefix(service, id_prefix)?;
    let tags = if remove {
        remove_tag_from_note(&note, tag)
    } else {
        add_tag_to_note(&note, tag)
    };

    if tags == note.tags {
        output::success("标签集合未发生变化");
        return Ok(());
    }

    let updated = service.edit_note(&note.id, note.content.clone(), tags)?;
    if remove {
        output::success(&format!(
            "已移除标签 #{}，新版本: {}",
            tag.trim(),
            &updated.id[..8]
        ));
    } else {
        output::success(&format!(
            "已添加标签 #{}，新版本: {}",
            tag.trim(),
            &updated.id[..8]
        ));
    }

    Ok(())
}

pub fn execute_add(
    id_prefix: &str,
    tag: &str,
    service: &SynapService,
) -> Result<(), Box<dyn std::error::Error>> {
    execute(id_prefix, tag, false, service)
}

pub fn execute_remove(
    id_prefix: &str,
    tag: &str,
    service: &SynapService,
) -> Result<(), Box<dyn std::error::Error>> {
    execute(id_prefix, tag, true, service)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_add_smoke() {
        let service = SynapService::open_memory().unwrap();
        let note = service.create_note("hello".to_string(), vec![]).unwrap();
        let result = execute_add(&note.id[..8], "rust", &service);
        assert!(result.is_ok());
    }
}
