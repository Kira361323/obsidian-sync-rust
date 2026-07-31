use super::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncStatus {
    UpToDate,
    Pushed,
    Pulled,
    Bidirectional,
    Committed,
    Conflict,
    Offline,
    Error,
    Skipped,
}

impl SyncStatus {
    pub fn badge(&self) -> &'static str {
        match self {
            Self::UpToDate => "[ OK ]",
            Self::Pushed => "[ UP ]",
            Self::Pulled => "[ DOWN ]",
            Self::Bidirectional => "[ SYNC ]",
            Self::Committed => "[ SAVE ]",
            Self::Conflict => "[ CONF ]",
            Self::Offline => "[ OFF ]",
            Self::Error => "[ FAIL ]",
            Self::Skipped => "[ DRY ]",
        }
    }

    pub fn color<'a>(&self, t: &'a Theme) -> &'a str {
        match self {
            Self::UpToDate => t.green(),
            Self::Pushed => t.cyan(),
            Self::Pulled => t.blue(),
            Self::Bidirectional => t.magenta(),
            Self::Committed => t.yellow(),
            Self::Conflict => t.yellow(),
            Self::Offline => t.dim(),
            Self::Error => t.red(),
            Self::Skipped => t.dim(),
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::UpToDate => "всё актуально",
            Self::Pushed => "отправлено на remote",
            Self::Pulled => "получено с remote",
            Self::Bidirectional => "двунаправленная синхронизация",
            Self::Committed => "локальный коммит",
            Self::Conflict => "конфликт, оставлена локальная версия",
            Self::Offline => "офлайн, remote-операции пропущены",
            Self::Error => "ошибка",
            Self::Skipped => "пропущено",
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(
            self,
            Self::UpToDate | Self::Pushed | Self::Pulled | Self::Bidirectional | Self::Committed
        )
    }

    pub fn is_warn(&self) -> bool {
        matches!(self, Self::Conflict | Self::Offline)
    }
}