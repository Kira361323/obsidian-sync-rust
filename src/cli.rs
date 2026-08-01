use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "obsidian-sync", version, about = "Obsidian Git Sync v4.1")]
pub struct Cli {
    /// Без вывода в консоль, только лог
    #[arg(long, global = true)]
    pub silent: bool,

    /// Компактный вывод
    #[arg(long, global = true)]
    pub compact: bool,

    /// Показать планируемые действия без изменений
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Тема: amoled | classic | mono | none
    #[arg(long, global = true, default_value = "amoled")]
    pub theme: String,

    /// Синхронизировать только указанный репозиторий (путь или имя)
    #[arg(long, global = true)]
    pub repo: Option<String>,

    /// Корень хранилищ (переопределяет OBSIDIAN_ROOT / default)
    #[arg(long, global = true)]
    pub root: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Полная синхронизация всех репозиториев
    Sync,
    /// Git push
    Push,
    /// Git pull
    Pull,
    /// Git commit локальных изменений
    Commit,
    /// Интерактивный разбор конфликтов
    Conflicts,
    /// Состояние репозиториев (local/remote)
    Status,
    /// Циклический sync с интервалом (фон при живом терминале)
    Watch {
        /// Интервал, напр. 15m / 1h / 30s
        #[arg(long, default_value = "15m")]
        interval: String,
    },
    /// Проверить и обновить утилиту
    Update,
    /// Проверить/обновить системный Git
    GitCheck {
        /// Обновить Git (с подтверждением)
        #[arg(long)]
        upgrade: bool,
    },
}

impl Cli {
    pub fn root(&self) -> PathBuf {
        if let Some(r) = &self.root {
            return r.clone();
        }
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