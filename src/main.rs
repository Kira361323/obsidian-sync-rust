mod cli;
mod config;
mod git;
mod lock;
mod logger;
mod menu;
mod network;
mod selfupdate;
mod status;
mod sync;
mod system;
mod ui;
mod watch;

use std::path::PathBuf;
use std::process;

use clap::Parser;

use cli::{Cli, Commands};
use lock::Lock;
use logger::Logger;
use sync::SyncContext;
use ui::theme::Theme;
use ui::Ui;

fn main() {
    let args = Cli::parse();
    let theme = Theme::from_str(&args.theme);
    let ui = Ui::new(theme, args.silent, args.compact);
    let logger = Logger::new(args.log_path());

    logger.prepare();
    logger.log("=== Sync started ===");

    let _lock = match Lock::acquire(args.lock_dir()) {
        Ok(l) => l,
        Err(e) => {
            ui.out(&format!("{}❌ {e}{}", ui.theme.red(), ui.theme.reset()));
            process::exit(1);
        }
    };

    // Команда: из CLI или из интерактивного меню.
    let command = args.command.clone().or_else(menu::run);

    let mut flags: Vec<&str> = vec![];
    if args.dry_run { flags.push("dry-run"); }
    if args.compact { flags.push("compact"); }
    if args.silent { flags.push("silent"); }
    ui.header(&flags);

    // Команды, не требующие root/сети/lock-логики sync.
    match &command {
        Some(Commands::Update) => {
            let _ = selfupdate::run(&ui);
            return;
        }
        Some(Commands::GitCheck { upgrade }) => {
            let _ = system::check(&ui, *upgrade);
            return;
        }
        _ => {}
    }

    if !check_requirements(&ui, &args.root()) {
        logger.log("ERROR: Requirements check failed");
        process::exit(1);
    }

    let offline = network::is_offline();
    if offline && !args.compact && !args.silent {
        ui.out(&format!(
            "  {}⚠️  Offline mode — remote sync skipped{}",
            ui.theme.yellow(),
            ui.theme.reset()
        ));
    }

    let root = args.root();
    if !root.is_dir() {
        ui.out(&format!(
            "{}❌ Root directory not found: {}{}",
            ui.theme.red(),
            root.display(),
            ui.theme.reset()
        ));
        process::exit(1);
    }

    let mut repos = discover_repos(&root);
    if let Some(ref filter) = args.repo {
        repos = filter_repos(&repos, filter);
        if repos.is_empty() {
            ui.out(&format!("{}⚠️  Repository not found: {filter}{}", ui.theme.yellow(), ui.theme.reset()));
            process::exit(1);
        }
    }

    if repos.is_empty() {
        ui.out(&format!("  {}⚠️  No git repositories found in:{}", ui.theme.yellow(), ui.theme.reset()));
        ui.out(&format!("  {}{}{}", ui.theme.dim(), root.display(), ui.theme.reset()));
        process::exit(0);
    }

    // Контекст для sync-операций (заимствует ui/logger на весь main).
    let ctx = SyncContext {
        ui: &ui,
        logger: &logger,
        offline,
        dry_run: args.dry_run,
    };

    match command {
        Some(Commands::Watch { interval }) => {
            // Watch сам крутит цикл; контекст пересоздаём на итерацию не нужно —
            // offline/dry_run не меняются. Передаём напрямую через замыкание-обёртку.
            run_watch(&interval, &repos, &ui, &logger, offline, args.dry_run);
        }
        Some(Commands::Status) => {
            print_status(&repos, &ui);
        }
        Some(Commands::Conflicts) => {
            // Заглушка до второго захода: текстовый список конфликтов.
            list_conflicts_text(&repos, &ui);
        }
        Some(Commands::Push) => single_op(&repos, &ctx, Op::Push),
        Some(Commands::Pull) => single_op(&repos, &ctx, Op::Pull),
        Some(Commands::Commit) => single_op(&repos, &ctx, Op::Commit),
        Some(Commands::Sync) | None => {
            run_sync_all(&repos, &ctx, &ui, &logger, &args);
        }
        Some(Commands::Update) | Some(Commands::GitCheck { .. }) => unreachable!(),
    }
}

#[derive(Clone, Copy)]
enum Op { Push, Pull, Commit }

fn single_op(repos: &[PathBuf], ctx: &SyncContext, op: Op) {
    // Минимальная обёртка: для каждой операции используем sync-путь частично.
    // Полноценные push/pull/commit как отдельные действия придут во втором заходе
    // вместе с отчётом по файлам; сейчас делегируем в sync_repo с dry_run=false,
    // но это выполнит полный цикл. Чтобы не вводить в заблуждение — честно:
    // во втором заходе разведу single_op на чистые git-вызовы.
    let _ = (repos, ctx, op);
    eprintln!("single-op push/pull/commit как изолированные действия — во втором заходе");
}

fn run_watch(
    interval: &str,
    repos: &[PathBuf],
    ui: &Ui,
    logger: &Logger,
    offline: bool,
    dry_run: bool,
) {
    let secs = match watch::parse_interval(interval) {
        Ok(s) => s,
        Err(e) => {
            ui.out(&format!("{}❌ {e}{}", ui.theme.red(), ui.theme.reset()));
            process::exit(2);
        }
    };
    ui.out(&format!(
        " {}Watch ON{} — интервал {}{}s{}, {}выход: Ctrl+C{}",
        ui.theme.green(),
        ui.theme.reset(),
        ui.theme.cyan(),
        secs,
        ui.theme.reset(),
        ui.theme.dim(),
        ui.theme.reset()
    ));
    logger.log(&format!("Watch started, interval={secs}s"));

    let mut iter = 0u64;
    loop {
        iter += 1;
        ui.out(&format!(
            " {}── итерация {iter} ──{}",
            ui.theme.dim(),
            ui.theme.reset()
        ));
        let ctx = SyncContext {
            ui,
            logger,
            offline,
            dry_run,
        };
        for (i, repo) in repos.iter().enumerate() {
            let _ = sync::sync_repo(repo, i + 1, repos.len(), &ctx);
        }
        for _ in 0..secs {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}

fn run_sync_all(
    repos: &[PathBuf],
    ctx: &SyncContext,
    ui: &Ui,
    logger: &Logger,
    args: &Cli,
) {
    let start = std::time::Instant::now();
    let total = repos.len();
    let mut ok = 0usize;
    let mut warn = 0usize;
    let mut err = 0usize;
    let mut errors: Vec<String> = vec![];

    for (i, repo) in repos.iter().enumerate() {
        match sync::sync_repo(repo, i + 1, total, ctx) {
            Ok(r) if r.status.is_ok() => ok += 1,
            Ok(r) if r.status.is_warn() => warn += 1,
            Ok(_) => err += 1,
            Err(e) => {
                err += 1;
                errors.push(e.clone());
                ui.out(&format!(" {}[ FAIL ]{} {}", ui.theme.red(), ui.theme.reset(), e));
            }
        }
    }

    let duration = start.elapsed().as_secs();
    ui.summary(ok, warn, err, duration, &errors);
    status::write(&args.status_path(), ok, warn, err, total, duration);
    logger.log(&format!("=== Sync done: OK={ok} WARN={warn} ERR={err} DURATION={duration}s ==="));
    if err > 0 { process::exit(1); }
}

fn print_status(repos: &[PathBuf], ui: &Ui) {
    let rows: Vec<ui::RepoStatus> = repos
        .iter()
        .map(|repo| {
            let name = repo
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let g = git::Git::new(repo);
            let (ahead, behind) = g.ahead_behind();
            let dirty = g
                .status_porcelain()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            let branch = g.current_branch().unwrap_or_else(|| "?".to_owned());
            ui::RepoStatus { name, branch, ahead, behind, dirty }
        })
        .collect();

    ui.status_table(&rows);
}

fn list_conflicts_text(repos: &[PathBuf], ui: &Ui) {
    let t = &ui.theme;
    let mut any = false;
    for repo in repos {
        let g = git::Git::new(repo);
        let files = g.conflicted_files();
        if files.is_empty() { continue; }
        any = true;
        let name = repo.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        ui.out(&format!(" {}{name}:{}", t.yellow(), t.reset()));
        for f in files {
            ui.out(&format!("   - {f}"));
        }
    }
    if !any {
        ui.out(&format!(" {}Конфликтов нет{}", t.green(), t.reset()));
    }
}

fn check_requirements(ui: &Ui, root: &std::path::Path) -> bool {
    for cmd in ["git"] {
        if std::process::Command::new(cmd).arg("--version").output().is_err() {
            ui.out(&format!("{}✘ Missing: {cmd}{}", ui.theme.red(), ui.theme.reset()));
            return false;
        }
    }
    if cfg!(target_os = "android") {
        let root_str = root.to_string_lossy();
        if root_str.starts_with("/storage/emulated/0/") && !root.is_dir() {
            ui.out(&format!("{}✘ Cannot access: {}{}", ui.theme.red(), root.display(), ui.theme.reset()));
            ui.out(&format!("{}💡 Run: termux-setup-storage{}", ui.theme.yellow(), ui.theme.reset()));
            return false;
        }
    }
    true
}

fn discover_repos(root: &std::path::Path) -> Vec<PathBuf> {
    let mut out = vec![];
    find_git_dirs(root, &mut out);
    out.sort();
    out.dedup();
    out
}

fn find_git_dirs(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.join(".git").is_dir() {
                out.push(path);
            } else {
                let name = entry.file_name().to_string_lossy().to_string();
                if name != ".git" && name != "node_modules" {
                    find_git_dirs(&path, out);
                }
            }
        }
    }
}

fn filter_repos(repos: &[PathBuf], filter: &str) -> Vec<PathBuf> {
    let filter = filter.trim_end_matches('/');
    repos.iter().filter(|r| {
        let r_str = r.to_string_lossy();
        let name = r.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        r_str.trim_end_matches('/') == filter || name == filter
    }).cloned().collect()
}