use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::config::CONFLICT_DIR;
use crate::git::Git;
use crate::logger::Logger;
use crate::ui::badge::SyncStatus;
use crate::ui::spinner::Spinner;
use crate::ui::Ui;

pub struct SyncResult {
    pub status: SyncStatus,
    pub elapsed: u64,
}

pub struct SyncContext<'a> {
    pub ui: &'a Ui,
    pub logger: &'a Logger,
    pub offline: bool,
    pub dry_run: bool,
}

pub fn sync_repo(
    repo: &Path,
    idx: usize,
    total: usize,
    ctx: &SyncContext,
) -> Result<SyncResult, String> {
    let name = repo
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| repo.display().to_string());

    let git = Git::new(repo);
    let start = Instant::now();

    if !git.is_repo() {
        let err = format!("{name}: Отсутствует папка .git");
        ctx.logger.log(&format!("ERROR: Not a git repo: {}", repo.display()));
        return Err(err);
    }

    git.ensure_safe_directory();
    git.ensure_conflict_dir_ignored(CONFLICT_DIR);
    git.untrack_conflict_dir(CONFLICT_DIR);

    let mut spinner: Option<Spinner> = None;
    if !ctx.ui.silent && ctx.ui.use_color {
        spinner = Some(Spinner::start(
            &format!("Обработка {name}..."),
            ctx.ui.theme.dim(),
            ctx.ui.theme.reset(),
        ));
    }

    let sync_info = build_sync_info(&git);

    let local_changes = match git.status_porcelain() {
        Ok(s) => s,
        Err(e) => {
            spinner = None;
            let err = format!("{name}: git status: {e}");
            ctx.logger.log(&format!("ERROR: {err}"));
            return Err(err);
        }
    };

    let mut remote_ok = !ctx.offline;
    if !ctx.dry_run && !ctx.offline {
        if let Err(e) = git.fetch() {
            remote_ok = false;
            ctx.logger
                .log(&format!("WARNING: Fetch failed in {name}: {e}"));
        }
    }

    let (ahead, behind) = git.ahead_behind();

    // ----------------------------------------------------------------- DRY-RUN
    if ctx.dry_run {
        spinner = None;
        let (status, plan) = dry_run_plan(&local_changes, remote_ok, ahead, behind);
        let elapsed = start.elapsed().as_secs();

        ctx.ui.repo_result(
            idx,
            total,
            &name,
            SyncStatus::Skipped,
            &sync_info,
            elapsed,
            Some(&format!("Dry-run: {plan}")),
        );

        return Ok(SyncResult { status, elapsed });
    }

    // ----------------------------------------------------------------- COMMIT
    let mut did_commit = false;
    let mut status = SyncStatus::UpToDate;

    if !local_changes.trim().is_empty() {
        if let Err(e) = git.ensure_identity() {
            spinner = None;
            return Err(format!("{name}: {e}"));
        }

        if let Err(e) = git.add_all() {
            spinner = None;
            return Err(format!("{name}: git add: {e}"));
        }

        let msg = format!(
            "📱 Mobile sync {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        );

        match git.commit(&msg) {
            Ok(()) => {
                did_commit = true;
                status = SyncStatus::Committed;
                ctx.logger.log(&format!("Committed local changes in {name}"));
            }
            Err(_) => {
                let recheck = git.status_porcelain().unwrap_or_default();
                if !recheck.trim().is_empty() {
                    spinner = None;
                    return Err(format!("{name}: git commit failed"));
                }
            }
        }
    }

    let (mut ahead, mut behind) = git.ahead_behind();

    // --------------------------------------------------------- INITIAL PUSH
    let mut did_push = false;
    if remote_ok && !git.has_upstream() {
        if let (Some(remote), Some(branch)) = (git.first_remote(), git.current_branch()) {
            if git.remote_branch_exists(&remote, &branch) {
                let _ = git.set_upstream_to(&remote, &branch);
                let (a, b) = git.ahead_behind();
                ahead = a;
                behind = b;
            } else {
                match git.push_set_upstream(&remote) {
                    Ok(_) => {
                        did_push = true;
                        status = SyncStatus::Pushed;
                        ctx.logger
                            .log(&format!("Pushed new branch to {remote} in {name}"));
                        let (a, b) = git.ahead_behind();
                        ahead = a;
                        behind = b;
                    }
                    Err(e) => {
                        spinner = None;
                        return Err(format!("{name}: Push ➔ {e}"));
                    }
                }
            }
        }
    }

    // ----------------------------------------------------------------- PULL
    let mut did_pull = false;
    let mut had_conflict = false;

    if behind > 0 && remote_ok {
        if ahead > 0 {
            if let Err(e) = git.ensure_identity() {
                spinner = None;
                return Err(format!("{name}: {e}"));
            }
        }

        match git.pull() {
            Ok(_) => {
                did_pull = true;
                status = SyncStatus::Pulled;
                ctx.logger.log(&format!("Pulled {behind} commit(s) in {name}"));
            }
            Err(_) => {
                let conflicted = git.conflicted_files();
                if !conflicted.is_empty() {
                    spinner = None;
                    match resolve_keep_local(&git, repo, ctx) {
                        Ok(()) => {
                            had_conflict = true;
                            did_pull = true;
                            status = SyncStatus::Conflict;
                            ctx.logger.log(&format!("Conflict auto-resolved in {name}"));
                        }
                        Err(e) => {
                            git.merge_abort();
                            return Err(format!("{name}: {e}"));
                        }
                    }
                    if !ctx.ui.silent && ctx.ui.use_color {
                        spinner = Some(Spinner::start(
                            &format!("Обработка {name}..."),
                            ctx.ui.theme.dim(),
                            ctx.ui.theme.reset(),
                        ));
                    }
                } else {
                    spinner = None;
                    return Err(format!("{name}: Pull failed"));
                }
            }
        }
    }

    if did_pull || had_conflict {
        let (a, _) = git.ahead_behind();
        ahead = a;
    }

    // ----------------------------------------------------------------- PUSH
    if ahead > 0 && remote_ok {
        match git.push() {
            Ok(_) => {
                did_push = true;
                if !had_conflict {
                    status = SyncStatus::Pushed;
                }
                ctx.logger.log(&format!("Pushed {ahead} commit(s) in {name}"));
            }
            Err(e) => {
                spinner = None;
                return Err(format!("{name}: Push ➔ {e}"));
            }
        }
    }

    spinner = None;

    // ----------------------------------------------------------------- FINAL
    if had_conflict {
        status = SyncStatus::Conflict;
    } else if did_pull && did_push {
        status = SyncStatus::Bidirectional;
    } else if !remote_ok && !did_commit && !did_pull && !did_push {
        status = SyncStatus::Offline;
    }

    let elapsed = start.elapsed().as_secs();
    ctx.ui
        .repo_result(idx, total, &name, status, &sync_info, elapsed, None);

    Ok(SyncResult { status, elapsed })
}

fn build_sync_info(git: &Git) -> String {
    let log = git.log_recent(20);
    let mut android_time = String::new();
    let mut pc_time = String::new();

    for (date, msg) in &log {
        if android_time.is_empty()
            && (msg.contains('📱') || msg.contains("Mobile") || msg.contains("Termux"))
        {
            android_time = date.clone();
        }
        if pc_time.is_empty()
            && (msg.contains('💻')
                || msg.contains("PC")
                || msg.contains("Vault backup")
                || msg.contains("backup"))
        {
            pc_time = date.clone();
        }
        if !android_time.is_empty() && !pc_time.is_empty() {
            break;
        }
    }

    let mut info = String::new();
    if !android_time.is_empty() {
        info.push_str(&format!("📱 {android_time}"));
    }
    if !pc_time.is_empty() {
        if !info.is_empty() {
            info.push_str("  ");
        }
        info.push_str(&format!("💻 {pc_time}"));
    }
    if info.is_empty() {
        info = "Нет истории синхронизации".to_owned();
    }
    info
}

fn dry_run_plan(
    local_changes: &str,
    remote_ok: bool,
    ahead: u32,
    behind: u32,
) -> (SyncStatus, String) {
    let would_commit = !local_changes.trim().is_empty();
    let would_pull = remote_ok && behind > 0;
    let would_push = remote_ok && (ahead > 0 || would_commit);

    if would_commit && would_pull {
        (SyncStatus::Bidirectional, "commit + pull + push".into())
    } else if would_commit && would_push {
        (SyncStatus::Committed, "commit + push".into())
    } else if would_commit {
        (SyncStatus::Committed, "commit".into())
    } else if would_pull && would_push {
        (SyncStatus::Bidirectional, "pull + push".into())
    } else if would_pull {
        (SyncStatus::Pulled, "pull".into())
    } else if would_push {
        (SyncStatus::Pushed, "push".into())
    } else {
        (SyncStatus::UpToDate, "без изменений".into())
    }
}

fn resolve_keep_local(git: &Git, repo: &Path, ctx: &SyncContext) -> Result<(), String> {
    let conflicted = git.conflicted_files();
    if conflicted.is_empty() {
        return Ok(());
    }

    backup_conflicts(git, repo, ctx);

    for f in &conflicted {
        let spec = format!(":2:{f}");
        if git.cat_file_exists(&spec) {
            let _ = git.checkout_ours(f);
            let _ = git.add_file(f);
        } else {
            let _ = git.rm_file(f);
        }
    }

    if !git.conflicted_files().is_empty() {
        return Err("Не удалось разрешить конфликт до чистого индекса".to_owned());
    }

    git.ensure_identity()?;
    git.commit("🔧 Auto-resolve conflict (keep local)")
        .map_err(|e| format!("Commit after conflict resolve: {e}"))
}

fn backup_conflicts(git: &Git, repo: &Path, ctx: &SyncContext) {
    let conflicted = git.conflicted_files();
    if conflicted.is_empty() {
        return;
    }

    let conflict_dir = repo.join(CONFLICT_DIR);
    let _ = fs::create_dir_all(&conflict_dir);

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");

    for f in &conflicted {
        let safe_name = f.replace('/', "__").replace('\n', "_");
        let base = conflict_dir.join(format!("{safe_name}_{ts}"));

        for (stage, suffix) in [(2, "ours"), (3, "theirs")] {
            if let Some(data) = git.show_stage(stage, f) {
                let path = base.with_extension(suffix);
                let _ = fs::write(&path, &data);
                ctx.logger
                    .log(&format!("Conflict backup: {f} → {}", path.display()));
            }
        }
    }

    cleanup_old_conflicts(&conflict_dir);
}

fn cleanup_old_conflicts(dir: &Path) {
    let cutoff =
        chrono::Local::now() - chrono::Duration::days(crate::config::CONFLICT_MAX_AGE_DAYS);

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    let modified: chrono::DateTime<chrono::Local> = modified.into();
                    if modified < cutoff {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
}