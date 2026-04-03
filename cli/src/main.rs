//! Synap CLI - a command line client for the current Synap core service.

mod commands;
mod config;
mod net;
mod output;
mod support;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use colored::Colorize;
use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};
use synap_core::SynapService;

#[derive(Parser)]
#[command(name = "synap")]
#[command(author = "Synap Contributors")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Synap CLI", long_about = None)]
struct Cli {
    #[arg(short, long, global = true)]
    db: Option<String>,

    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(group = "mode", value_name = "CONTENT", num_args = 1..)]
    capture: Option<Vec<String>>,

    #[command(subcommand)]
    command: Option<Commands>,
}

fn looks_like_note_id_prefix(s: &str) -> bool {
    let len = s.len();
    (4..=36).contains(&len) && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

#[derive(Subcommand)]
enum Commands {
    Amend {
        id: String,
    },
    Void {
        id: String,
    },
    Trace {
        id: Option<String>,
        #[arg(long)]
        stream: bool,
    },
    Search {
        query: String,
    },
    Scrub,
    Show {
        id: String,
    },
    Stats,
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },
    List {
        tag: Option<String>,
    },
    Tag {
        action: String,
        id: String,
        tag: String,
    },
    Graph {
        id: Option<String>,
    },
    Completions {
        shell: String,
    },
    InstallCompletions,
}

#[derive(Subcommand)]
enum SyncAction {
    Init { addr: String },
    Respond,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some(Commands::Completions { shell }) = &cli.command {
        handle_completions(shell)?;
        return Ok(());
    }

    if matches!(cli.command, Some(Commands::InstallCompletions)) {
        install_completions()?;
        return Ok(());
    }

    let db_path = config::resolve_db_path(cli.db.clone());
    config::ensure_db_dir_exists_for(&db_path)?;

    let service = SynapService::open(&db_path).map_err(|e| {
        format!(
            "{} 无法打开数据库 {}: {}",
            "[ERROR]".red(),
            db_path.display(),
            e
        )
    })?;

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Amend { id } => commands::amend::execute(&id, &service),
            Commands::Void { id } => commands::void::execute(&id, &service),
            Commands::Trace { id, stream } => {
                if stream {
                    commands::trace::execute_stream(&service)
                } else {
                    commands::trace::execute(id.as_deref(), &service)
                }
            }
            Commands::Search { query } => commands::search::execute(&query, &service),
            Commands::Scrub => commands::scrub::execute(&service),
            Commands::Show { id } => commands::show::execute(&id, &service),
            Commands::Stats => commands::stats::execute(&service),
            Commands::Sync { action } => match action {
                SyncAction::Init { addr } => commands::sync::execute(
                    &["sync".to_string(), "init".to_string(), addr],
                    &service,
                ),
                SyncAction::Respond => {
                    commands::sync::execute(&["sync".to_string(), "respond".to_string()], &service)
                }
            },
            Commands::List { tag } => commands::list::execute(&service, tag),
            Commands::Tag { action, id, tag } => match action.as_str() {
                "add" => commands::tag::execute_add(&id, &tag, &service),
                "remove" => commands::tag::execute_remove(&id, &tag, &service),
                _ => Err("无效的 tag 操作: add 或 remove".into()),
            },
            Commands::Graph { id } => commands::graph::execute(id.as_deref(), &service),
            Commands::Completions { .. } => unreachable!(),
            Commands::InstallCompletions => unreachable!(),
        }
    } else if let Some(args) = cli.capture {
        if let Some(first) = args.first() {
            if first.starts_with('@') && args.len() >= 2 {
                let id = first.trim_start_matches('@');
                let content = args[1..].join(" ");
                commands::reply::execute(id, &content, &service)
            } else if first.starts_with('#') {
                let tag = first.trim_start_matches('#');
                commands::search::execute_tag(tag, &service)
            } else if args.len() == 1 && looks_like_note_id_prefix(first) {
                commands::show::execute(first, &service)
            } else {
                let content = args.join(" ");
                commands::capture::execute(&content, &service)
            }
        } else {
            commands::trace::execute_recent(&service)
        }
    } else {
        commands::trace::execute_recent(&service)
    }
}

fn handle_completions(shell: &str) -> Result<(), Box<dyn std::error::Error>> {
    let shell = match shell {
        "bash" => Shell::Bash,
        "elvish" => Shell::Elvish,
        "fish" => Shell::Fish,
        "powershell" | "pwsh" => Shell::PowerShell,
        "zsh" => Shell::Zsh,
        _ => return Err(format!("不支持的 shell 类型: {}", shell).into()),
    };

    print_completions(shell, &mut Cli::command());
    Ok(())
}

fn print_completions<G: Generator>(generator: G, cmd: &mut clap::Command) {
    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut io::stdout(),
    );
}

fn install_completions() -> Result<(), Box<dyn std::error::Error>> {
    let shell = detect_shell()?;
    let (install_dir, filename, config_file, config_snippet) = match shell {
        Shell::Zsh => {
            let dir = env::var("HOME")
                .map(PathBuf::from)
                .map(|p| p.join(".zsh").join("completion"))
                .unwrap_or_else(|_| PathBuf::from("/usr/local/share/zsh/site-functions"));

            let rc_file = env::var("HOME")
                .map(PathBuf::from)
                .map(|p| p.join(".zshrc"))
                .unwrap();

            let snippet =
                "\n# Synap completions\nfpath=($HOME/.zsh/completion $fpath)\n".to_string();
            (dir, "_synap", Some(rc_file), Some(snippet))
        }
        Shell::Fish => {
            let dir = env::var("HOME")
                .map(PathBuf::from)
                .map(|p| p.join(".config").join("fish").join("completions"))
                .unwrap();
            (dir, "synap.fish", None, None)
        }
        Shell::Bash => {
            let dir = env::var("HOME")
                .map(PathBuf::from)
                .map(|p| {
                    p.join(".local")
                        .join("share")
                        .join("bash-completion")
                        .join("completions")
                })
                .unwrap();

            let rc_file = env::var("HOME")
                .map(PathBuf::from)
                .map(|p| p.join(".bashrc"))
                .unwrap();

            let snippet =
                "\n# Synap completions\nsource ~/.local/share/bash-completion/completions/synap 2>/dev/null\n"
                    .to_string();
            (dir, "synap", Some(rc_file), Some(snippet))
        }
        _ => {
            return Err(format!("{}: {:?}", "不支持的 shell".red(), shell).into());
        }
    };

    fs::create_dir_all(&install_dir)?;

    let file_path = install_dir.join(filename);
    let mut file = File::create(&file_path)?;
    generate(shell, &mut Cli::command(), "synap".to_string(), &mut file);

    println!(
        "{} {}",
        "✓ 补全脚本已安装到:".green(),
        file_path.display().to_string().cyan()
    );

    if let (Some(rc_file), Some(snippet)) = (config_file, config_snippet) {
        if rc_file.exists() {
            let content = fs::read_to_string(&rc_file)?;
            if !content.contains("Synap completions") {
                let mut file = File::options().append(true).open(&rc_file)?;
                file.write_all(snippet.as_bytes())?;
            }
        } else {
            let mut file = File::create(&rc_file)?;
            file.write_all(snippet.as_bytes())?;
        }
    }

    println!(
        "\n{}",
        "重新加载配置或重启 shell 后即可使用自动补全".yellow()
    );
    Ok(())
}

fn detect_shell() -> Result<Shell, Box<dyn std::error::Error>> {
    if let Ok(shell_path) = env::var("SHELL") {
        if shell_path.contains("zsh") {
            return Ok(Shell::Zsh);
        }
        if shell_path.contains("fish") {
            return Ok(Shell::Fish);
        }
        if shell_path.contains("bash") {
            return Ok(Shell::Bash);
        }
    }

    Err("无法检测到支持的 shell，请手动指定 bash/fish/zsh".into())
}
