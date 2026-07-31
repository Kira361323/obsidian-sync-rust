#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    Amoled,
    Classic,
    Mono,
    None,
}

impl Theme {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "amoled" => Self::Amoled,
            "classic" => Self::Classic,
            "mono" => Self::Mono,
            "none" => Self::None,
            _ => Self::Amoled,
        }
    }

    pub fn red(&self) -> &'static str {
        match self {
            Self::Amoled => "\x1b[1;91m",
            Self::Classic => "\x1b[0;31m",
            Self::Mono | Self::None => "",
        }
    }

    pub fn green(&self) -> &'static str {
        match self {
            Self::Amoled => "\x1b[1;92m",
            Self::Classic => "\x1b[0;32m",
            Self::Mono | Self::None => "",
        }
    }

    pub fn yellow(&self) -> &'static str {
        match self {
            Self::Amoled => "\x1b[1;93m",
            Self::Classic => "\x1b[1;33m",
            Self::Mono | Self::None => "",
        }
    }

    pub fn blue(&self) -> &'static str {
        match self {
            Self::Amoled => "\x1b[1;94m",
            Self::Classic => "\x1b[0;34m",
            Self::Mono | Self::None => "",
        }
    }

    pub fn cyan(&self) -> &'static str {
        match self {
            Self::Amoled => "\x1b[1;96m",
            Self::Classic => "\x1b[0;36m",
            Self::Mono | Self::None => "",
        }
    }

    pub fn magenta(&self) -> &'static str {
        match self {
            Self::Amoled => "\x1b[1;95m",
            Self::Classic => "\x1b[0;35m",
            Self::Mono | Self::None => "",
        }
    }

    pub fn dim(&self) -> &'static str {
        match self {
            Self::None => "",
            _ => "\x1b[2m",
        }
    }

    pub fn bold(&self) -> &'static str {
        match self {
            Self::None => "",
            _ => "\x1b[1m",
        }
    }

    pub fn reset(&self) -> &'static str {
        match self {
            Self::None => "",
            _ => "\x1b[0m",
        }
    }
}