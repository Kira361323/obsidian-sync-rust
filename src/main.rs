mod cli;
mod config;
mod git;
mod lock;
mod logger;
mod network;
mod status;
mod sync;
mod ui;

use std::path::PathBuf;
use std::process;

use clap::Parser;

use cli::Cli;
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

    // Lock
    let _lock = match Lock::acquire(args.lock_dir()) {
        Ok(l) => l,
        Err(e) => {
            ui.out(&format!("{}❌ {e}{}", ui.theme.red(), ui.theme.reset()));
            process::exit(1);
        }
    };

    // Header
    let mut flags: Vec<&str> = vec![];
    if args.dry_run {
        flags.push("dry-run");
    }
    if args.compact {
        flags.push("compact");
    }
    if args.silent {
        flags.push("silent");
    }
    ui.header(&flags);

    // Requirements
    if !check_requirements(&ui, &args.root()) {
        logger.log("ERROR: Requirements check failed");
        process::exit(1);
    }

    // Offline check
    let offline = network::is_offline();
    if offline {
        logger.log("Network appears offline — remote operations will be skipped");
        if !args.compact && !args.silent {
            ui.out(&format!(
                "  {}⚠️  Offline mode — remote sync skipped{}",
                ui.theme.yellow(),
                ui.theme.reset()
            ));
        }
    }

    // Discover repos
    let root = args.root();
    if !root.is_dir() {
        ui.out(&format!(
            "{}❌ Root directory not found: {}{}",
            ui.theme.red(),
            root.display(),
            ui.theme.reset()
        ));
        logger.log(&format!("ERROR: Root not found: {}", root.display()));
        process::exit(1);
    }

    let mut repos = discover_repos(&root);

    // Filter
    if let Some(ref filter) = args.repo {
        let filter = filter.trim_end_matches('/');
        let filtered: Vec<PathBuf> = repos
            .iter()
            .filter(|r| {
                let r_str = r.to_string_lossy();
                let name = r
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                r_str.trim_end_matches('/') == filter || name == filter
            })
            .cloned()
            .collect();

        if filtered.is_empty() {
            ui.out(&format!(
                "{}⚠️  Repository not found: {filter}{}",
                ui.theme.yellow(),
                ui.theme.reset()
            ));
            process::exit(1);
        }
        repos = filtered;
    }

    if repos.is_empty() {
        ui.out(&format!(
            "  {}⚠️  No git repositories found in:{}",
            ui.theme.yellow(),
            ui.theme.reset()
        ));
        ui.out(&format!("  {}{}{}", ui.theme.dim(), root.display(), ui.theme.reset()));
        logger.log(&format!("WARNING: No repos found in {}", root.display()));
    }

    // Sync
    let start = std::time::Instant::now();
    let total = repos.len();
    let mut total_ok = 0usize;
    let mut total_warn = 0usize;
    let mut total_err = 0usize;
    let mut errors: Vec<String> = vec![];

    let ctx = SyncContext {
        ui: &ui,
        logger: &logger,
        offline,
        dry_run: args.dry_run,
    };

    for (i, repo) in repos.iter().enumerate() {
        let idx = i + 1;
        match sync::sync_repo(repo, idx, total, &ctx) {
            Ok(result) => {
                if result.status.is_ok() {
                    total_ok += 1;
                } else if result.status.is_warn() {
                    total_warn += 1;
                } else {
                    total_err += 1;
                }
            }
            Err(e) => {
                total_err += 1;
                errors.push(e.clone());
                ui.out(&format!(
                    " {}[ FAIL ]{} {}",
                    ui.theme.red(),
                    ui.theme.reset(),
                    e
                ));
            }
        }
    }

    let duration = start.elapsed().as_secs();

    // Summary
    ui.summary(total_ok, total_warn, total_err, duration, &errors);
    status::write(&args.status_path(), total_ok, total_warn, total_err, total, duration);

    logger.log(&format!(
        "=== Sync done: OK={total_ok} WARN={total_warn} ERR={total_err} DURATION={duration}s ==="
    ));

    if total_err > 0 {
        process::exit(1);
    }
}

fn check_requirements(ui: &Ui, root: &std::path::Path) -> bool {
    let cmds = ["git", "find"];
    let mut missing = vec![];

    for cmd in &cmds {
        if std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| !o.status.success())
            .unwrap_or(true)
        {
            // Fallback: пробуем напрямую
            if std::process::Command::new(cmd)
                .arg("--version")
                .output()
                .is_err()
            {
                missing.push(*cmd);
            }
        }
    }

    if !missing.is_empty() {
        ui.out(&format!(
            "{}✘ Missing: {}{}",
            ui.theme.red(),
            missing.join(", "),
            ui.theme.reset()
        ));
        return false;
    }

    // Android-specific
    if cfg!(target_os = "android") {
        let root_str = root.to_string_lossy();
        if root_str.starts_with("/storage/emulated/0/") {
            if !root.is_dir() {
                ui.out(&format!(
                    "{}✘ Cannot access: {}{}",
                    ui.theme.red(),
                    root.display(),
                    ui.theme.reset()
                ));
                ui.out(&format!(
                    "{}💡 Run: termux-setup-storage{}",
                    ui.theme.yellow(),
                    ui.theme.reset()
                ));
                return false;
            }
        }
    }

    true
}

fn discover_repos(root: &std::path::Path) -> Vec<PathBuf> {
    let mut repos = vec![];
    find_git_dirs(root, &mut repos);
    repos.sort();
    repos.dedup();
    repos
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
                // Не рекурсируем в .git
                let name = entry.file_name().to_string_lossy().to_string();
                if name != ".git" && name != "node_modules" {
                    find_git_dirs(&path, out);
                }
            }
        }
    }
}