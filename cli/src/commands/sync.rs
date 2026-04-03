//! Sync command implementation - P2P synchronization.

use std::net::TcpListener;

use colored::Colorize;
use synap_core::{
    sync::{SyncConfig, SyncService},
    SynapService,
};

use crate::net::TcpConn;

pub fn execute(args: &[String], service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    let subcommand = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match subcommand {
        "init" => {
            let addr = args.get(2).ok_or("sync init 需要指定地址")?;
            execute_init(addr, service)
        }
        "respond" => execute_respond(service),
        _ => {
            print_sync_help();
            Err("未知的 sync 子命令".into())
        }
    }
}

fn execute_init(addr: &str, service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("{} 正在连接到 {}...", "[→]".dimmed(), addr.cyan());
    let mut conn = TcpConn::connect(addr)?;
    let sync = SyncService::new(service, SyncConfig::default());
    let stats = sync.sync_as_initiator(&mut conn)?;

    println!();
    println!("{} 同步完成", "[✓]".green());
    println!("发送记录: {}", stats.records_sent.to_string().cyan());
    println!("接收记录: {}", stats.records_received.to_string().cyan());
    println!("应用记录: {}", stats.records_applied.to_string().cyan());
    println!("跳过记录: {}", stats.records_skipped.to_string().dimmed());
    println!("发送字节: {}", stats.bytes_sent.to_string().dimmed());
    println!("接收字节: {}", stats.bytes_received.to_string().dimmed());
    println!("耗时: {} ms", stats.duration_ms.to_string().dimmed());
    println!();

    Ok(())
}

fn execute_respond(service: &SynapService) -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:8080";
    eprintln!("{} 正在监听 {}...", "[*]".blue(), addr.cyan());
    eprintln!("{} 按 Ctrl+C 停止监听", "[i]".dimmed());

    let listener = TcpListener::bind(addr)?;
    let sync = SyncService::new(service, SyncConfig::default());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer_addr = stream.peer_addr()?;
                eprintln!(
                    "{} 连接来自: {}",
                    "[+]".green(),
                    peer_addr.to_string().cyan()
                );
                let mut conn = TcpConn::from_stream(stream);
                let stats = sync.sync_as_responder(&mut conn)?;

                println!();
                println!("{} 同步完成", "[✓]".green());
                println!("发送记录: {}", stats.records_sent.to_string().cyan());
                println!("接收记录: {}", stats.records_received.to_string().cyan());
                println!("应用记录: {}", stats.records_applied.to_string().cyan());
                println!("跳过记录: {}", stats.records_skipped.to_string().dimmed());
                println!("发送字节: {}", stats.bytes_sent.to_string().dimmed());
                println!("接收字节: {}", stats.bytes_received.to_string().dimmed());
                println!("耗时: {} ms", stats.duration_ms.to_string().dimmed());
                println!();
            }
            Err(err) => eprintln!("{} 连接失败: {}", "[!]".red(), err),
        }
    }

    Ok(())
}

fn print_sync_help() {
    println!();
    println!("Sync 命令:");
    println!("  synap sync init <ADDR>     连接到对等节点并同步");
    println!("  synap sync respond         监听传入的同步连接");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_invalid_subcommand() {
        let service = SynapService::open_memory().unwrap();
        let result = execute(&["sync".to_string(), "invalid".to_string()], &service);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_init_missing_addr() {
        let service = SynapService::open_memory().unwrap();
        let result = execute(&["sync".to_string(), "init".to_string()], &service);
        assert!(result.is_err());
    }
}
