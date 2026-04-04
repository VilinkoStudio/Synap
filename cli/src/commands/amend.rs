//! Amend command implementation - immutable correction with editor integration.

use std::{
    env,
    io::{Read, Seek, SeekFrom, Write},
    process::Command,
};

use colored::Colorize;
use synap_core::SynapService;
use tempfile::NamedTempFile;

use crate::support::resolve_note_prefix;

pub fn execute(id_prefix: &str, service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    let old_note = resolve_note_prefix(service, id_prefix)?;

    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", old_note.content)?;

    let editor = detect_editor();
    println!("{} 正在打开 {}...", "[i]".blue(), editor);
    let status = Command::new(&editor).arg(temp_file.path()).status()?;

    if !status.success() {
        return Err(format!("{} 编辑器退出异常", "[ERROR]".red()).into());
    }

    temp_file.seek(SeekFrom::Start(0))?;
    let mut new_content = String::new();
    temp_file.read_to_string(&mut new_content)?;

    if new_content.trim().is_empty() {
        return Err(format!("{} 内容不能为空", "[ERROR]".red()).into());
    }

    if new_content == old_note.content {
        println!("{} 内容未变更，跳过修正", "[~]".yellow());
        return Ok(());
    }

    let new_note = service.edit_note(&old_note.id, new_content, old_note.tags.clone())?;

    println!(
        "{} 发生偏转: 新区块 ({}) ──[替代]─> ({})",
        "[~]".yellow(),
        new_note.id[..8].cyan(),
        old_note.id[..8].dimmed()
    );

    Ok(())
}

fn detect_editor() -> String {
    if let Ok(editor) = env::var("EDITOR") {
        return editor;
    }

    for editor in ["nvim", "vim", "vi", "nano", "emacs"] {
        if Command::new(editor).arg("--version").output().is_ok() {
            return editor.to_string();
        }
    }

    "vi".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_editor_with_env() {
        let original = env::var("EDITOR");
        env::set_var("EDITOR", "test-editor");
        let editor = detect_editor();
        assert_eq!(editor, "test-editor");

        match original {
            Ok(v) => env::set_var("EDITOR", v),
            Err(_) => env::remove_var("EDITOR"),
        }
    }

    #[test]
    fn test_detect_editor_fallback() {
        env::remove_var("EDITOR");
        let editor = detect_editor();
        assert!(!editor.is_empty());
    }
}
