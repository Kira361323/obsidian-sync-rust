use dialoguer::{Confirm, Input, Select};

use crate::cli::Commands;

const ITEMS: &[&str] = &[
    "Sync — полная синхронизация",
    "Push — отправить на remote",
    "Pull — получить с remote",
    "Commit — сохранить локальные изменения",
    "Conflicts — разобрать конфликты",
    "Status — состояние репозиториев",
    "Watch — фон (цикл по интервалу)",
    "Update — проверить/обновить утилиту",
    "Git — проверить/обновить Git",
    "Выход",
];

/// Возвращает выбранную подкоманду или None (выход).
pub fn run() -> Option<Commands> {
    loop {
        let sel = Select::new()
            .with_prompt("OBSIDIAN SYNC")
            .items(ITEMS)
            .default(0)
            .interact_opt()
            .unwrap_or(None)?;

        match sel {
            0 => return Some(Commands::Sync),
            1 => return Some(Commands::Push),
            2 => return Some(Commands::Pull),
            3 => return Some(Commands::Commit),
            4 => return Some(Commands::Conflicts),
            5 => return Some(Commands::Status),
            6 => {
                let on = Confirm::new()
                    .with_prompt("Запустить фон (цикл sync)?")
                    .default(true)
                    .interact()
                    .unwrap_or(false);
                if !on {
                    continue;
                }
                let interval: String = Input::new()
                    .with_prompt("Интервал (напр. 15m / 1h / 30s)")
                    .default("15m".to_owned())
                    .interact_text()
                    .unwrap_or_else(|_| "15m".to_owned());
                return Some(Commands::Watch { interval });
            }
            7 => return Some(Commands::Update),
            8 => {
                let upgrade = Confirm::new()
                    .with_prompt("Обновить системный Git?")
                    .default(false)
                    .interact()
                    .unwrap_or(false);
                return Some(Commands::GitCheck { upgrade });
            }
            9 => return None,
            _ => continue,
        }
    }
}