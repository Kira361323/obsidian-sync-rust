use std::thread;
use std::time::Duration;

use crate::logger::Logger;
use crate::sync::SyncContext;
use crate::ui::Ui;

/// Парсит "15m" / "1h" / "30s" / "90" (секунды) в секунды.
pub fn parse_interval(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("пустой интервал".to_owned());
    }
    let (num, mul) = if s.ends_with('h') {
        (&s[..s.len() - 1], 3600u64)
    } else if s.ends_with('m') {
        (&s[..s.len() - 1], 60u64)
    } else if s.ends_with('s') {
        (&s[..s.len() - 1], 1u64)
    } else {
        (s, 1u64)
    };
    let n: u64 = num
        .trim()
        .parse()
        .map_err(|_| format!("не число в интервале: {s}"))?;
    if n == 0 {
        return Err("интервал должен быть > 0".to_owned());
    }
    Ok(n * mul)
}

pub fn run(
    interval: &str,
    repos: &[std::path::PathBuf],
    ctx_factory: impl Fn() -> SyncContext<'static>,
    ui: &Ui,
    logger: &Logger,
) -> Result<(), String> {
    let secs = parse_interval(interval)?;
    let t = &ui.theme;

    ui.out(&format!(
        " {}Watch ON{} — интервал {}{}s{}, выход: Ctrl+C{}",
        t.green(),
        t.reset(),
        t.cyan(),
        secs,
        t.reset(),
        t.dim(),
        t.reset()
    ));
    logger.log(&format!("Watch started, interval={secs}s"));

    let mut iter = 0u64;
    loop {
        iter += 1;
        ui.out(&format!(
            " {}── итерация {iter} ──{}",
            t.dim(),
            t.reset()
        ));

        let ctx = ctx_factory();
        for (i, repo) in repos.iter().enumerate() {
            let _ = crate::sync::sync_repo(repo, i + 1, repos.len(), &ctx);
        }

        // Сон чипами по 1с — чтобы Ctrl+C убил promptly (default SIGINT handler).
        // Stale-lock после смерти чистится по PID при следующем запуске.
        for _ in 0..secs {
            thread::sleep(Duration::from_secs(1));
        }
    }
}