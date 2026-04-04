//! Scrub command implementation.

use colored::Colorize;
use synap_core::SynapService;

use crate::support::fetch_all_deleted;

pub fn execute(service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    let deleted = fetch_all_deleted(service)?;

    println!("{} 当前 core 仅支持逻辑删除", "[i]".blue());
    println!(
        "{} 已扫描到 {} 条墓碑笔记，尚未提供物理 scrub/compact 接口",
        "[i]".blue(),
        deleted.len().to_string().cyan()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_empty_db() {
        let service = SynapService::open_memory().unwrap();
        let result = execute(&service);
        assert!(result.is_ok());
    }
}
