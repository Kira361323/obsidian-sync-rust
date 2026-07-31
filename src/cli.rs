use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "obsidian-sync", version, about = "Obsidian Git Sync v4")]
pub struct Cli {
    /// Без вывода в консоль, только лог
    #[arg(long)]
    pub silent: bool,

    /// Компактный вывод
    #[arg(long)]
    pub compact: bool,

    /// Показать планируемые действия без изменений
    #[arg(long)]
    pub dry_run: bool,

    /// Тема: amoled | classic | mono | none
    #[arg(long, default_value = "amoled")]
    pub theme: String,

    /// Синхронизировать только указанный репозиторий (путь или имя)
    #[arg(long)]
    pub repo: Option<String>,
}

impl Cli {
    pub fn root(&self) -> PathBuf {
        std::env::var("OBSIDIAN_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                if cfg!(target_os = "android") {
                    PathBuf::from("/storage/emulated/0/Documents/Vaults")
                } else {
                    dirs::document_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join("Vaults")
                }
            })
    }

    pub fn log_path(&self) -> PathBuf {
        std::env::var("OBSIDIAN_LOG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join(".obsidian_sync.log")
            })
    }

    pub fn status_path(&self) -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".obsidian_sync_status")
    }

    pub fn lock_dir(&self) -> PathBuf {
        let base = std::env::var("PREFIX")
            .map(PathBuf::from)
            .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")));
        base.join("tmp").join("obsidian_sync.lock.d")
    }
}