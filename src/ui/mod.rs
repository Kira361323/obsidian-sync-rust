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
    width: usize,
}

impl Ui {
    pub fn new(theme: Theme, silent: bool, compact: bool) -> Self {
        let use_color = atty_stdout() && std::env::var("NO_COLOR").is_err();
        let theme = if use_color { theme } else { Theme::None };
        let width = detect_width();
        Self {
            theme,
            silent,
            compact,
            use_color,
            width,
        }
    }

    pub fn out(&self, msg: &str) {
        if !self.silent {
            println!("{msg}");
        }
    }

    pub fn sep(&self) -> String {
        let ch = if is_utf8_locale() { "─" } else { "-" };
        ch.repeat(self.width)
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
        let badge = format!("{}{}{}", status.color(t), status.badge(), t.reset());
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

/// Реальная ширина терминала в колонках, cap 78 (как в bash-оригинале).
/// Порядок: $COLUMNS → `stty size` (unix) → дефолт 80.
fn detect_width() -> usize {
    let raw = detect_width_raw();
    if raw == 0 { 78 } else { raw.min(78) }
}

fn detect_width_raw() -> usize {
    if let Ok(c) = std::env::var("COLUMNS") {
        if let Ok(n) = c.parse::<usize>() {
            if n > 0 {
                return n;
            }
        }
    }

    #[cfg(unix)]
    {
        if let Ok(out) = std::process::Command::new("stty").arg("size").output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                let mut it = s.trim().split_whitespace();
                let _rows = it.next();
                if let Some(cols) = it.next() {
                    if let Ok(n) = cols.parse::<usize>() {
                        if n > 0 {
                            return n;
                        }
                    }
                }
            }
        }
    }

    80
}

fn atty_stdout() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc_isatty(1) != 0 }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "isatty"]
    fn libc_isatty(fd: i32) -> i32;
}

fn is_utf8_locale() -> bool {
    let locale = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_CTYPE"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    locale.to_lowercase().contains("utf")
}