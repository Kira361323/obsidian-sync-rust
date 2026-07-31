pub mod badge;
pub mod spinner;
pub mod theme;

use badge::SyncStatus;
use theme::Theme;

pub struct Ui {
    pub theme: Theme,
    pub silent: bool,
    pub compact: bool,
    pub use_color: bool,
}

impl Ui {
    pub fn new(theme: Theme, silent: bool, compact: bool) -> Self {
        let use_color = atty_stdout() && std::env::var("NO_COLOR").is_err();
        let theme = if use_color { theme } else { Theme::None };
        Self {
            theme,
            silent,
            compact,
            use_color,
        }
    }

    pub fn out(&self, msg: &str) {
        if !self.silent {
            println!("{msg}");
        }
    }

    pub fn sep(&self) -> String {
        let w = term_width();
        let ch = if is_utf8_locale() { "─" } else { "-" };
        ch.repeat(w)
    }

    pub fn header(&self, flags: &[&str]) {
        let ts = chrono::Local::now().format("%H:%M  %d/%m/%Y");
        let t = &self.theme;
        let suffix = if flags.is_empty() {
            String::new()
        } else {
            format!("  {}({}){}", t.dim(), flags.join(", "), t.reset())
        };
        self.out("");
        self.out(&format!(
            " {}{}OBSIDIAN SYNC{}  {}{}{}{}",
            t.bold(),
            t.cyan(),
            t.reset(),
            t.dim(),
            ts,
            t.reset(),
            suffix
        ));
        self.out(&self.sep());
    }

    pub fn repo_result(
        &self,
        idx: usize,
        total: usize,
        name: &str,
        status: SyncStatus,
        sync_info: &str,
        elapsed: u64,
        note: Option<&str>,
    ) {
        let t = &self.theme;
        let badge = format!(
            "{}{}{}",
            status.color(t),
            status.badge(),
            t.reset()
        );
        let et = if elapsed > 0 {
            format!(" {}{}s{}", t.dim(), elapsed, t.reset())
        } else {
            String::new()
        };

        self.out(&format!(
            " {}[{}/{}]{} {}{}{} {}{}",
            t.dim(),
            idx,
            total,
            t.reset(),
            t.bold(),
            name,
            t.reset(),
            badge,
            et
        ));

        if !self.compact {
            self.out(&format!("        {}{}{}", t.dim(), sync_info, t.reset()));
            self.out(&format!(
                "        {}↳ {}{}",
                t.dim(),
                status.description(),
                t.reset()
            ));
        }

        if let Some(n) = note {
            self.out(&format!("        {}↳ {}{}", t.dim(), n, t.reset()));
        }

        if !self.compact {
            self.out("");
        }
    }

    pub fn summary(&self, ok: usize, warn: usize, err: usize, duration: u64, errors: &[String]) {
        let t = &self.theme;
        self.out(&self.sep());
        self.out(&format!(
            " {}ИТОГ{}  {}OK:{}{}  {}WARN:{}{}  {}FAIL:{}{}  {}TIME:{}s{}",
            t.bold(),
            t.reset(),
            t.green(),
            ok,
            t.reset(),
            t.yellow(),
            warn,
            t.reset(),
            t.red(),
            err,
            t.reset(),
            t.dim(),
            duration,
            t.reset()
        ));

        if !errors.is_empty() {
            self.out("");
            self.out(&format!(" {}{}Ошибки:{}", t.red(), t.bold(), t.reset()));
            for e in errors {
                self.out(&format!("  - {e}"));
            }
        }
        self.out("");
    }
}

fn atty_stdout() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc_isatty(1) != 0 }
    }
    #[cfg(not(unix))]
    {
        // Windows: упрощённо, всегда true если не перенаправлено
        true
    }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "isatty"]
    fn libc_isatty(fd: i32) -> i32;
}

fn term_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60)
        .min(78)
}

fn is_utf8_locale() -> bool {
    let locale = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_CTYPE"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    locale.to_lowercase().contains("utf")
}