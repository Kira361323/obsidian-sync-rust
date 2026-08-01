use std::path::Path;
use std::process::Command;

use crate::config::{FETCH_TIMEOUT_SECS, GIT_OP_TIMEOUT_SECS};

pub struct Git<'a> {
    repo: &'a Path,
}

impl<'a> Git<'a> {
    pub fn new(repo: &'a Path) -> Self {
        Self { repo }
    }

    fn base_cmd(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(self.repo)
            .arg("-c")
            .arg("core.hooksPath=/dev/null");
        cmd.env("GIT_OPTIONAL_LOCKS", "0");
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        cmd.env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes");
        cmd
    }

    fn run(&self, args: &[&str]) -> Result<String, String> {
        let out = self
            .base_cmd()
            .args(args)
            .output()
            .map_err(|e| format!("git {}: {e}", args.join(" ")))?;

        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).to_string())
        }
    }

    fn run_timeout(&self, args: &[&str], secs: u64) -> Result<String, String> {
        #[cfg(unix)]
        {
            let mut cmd = Command::new("timeout");
            cmd.arg(format!("{secs}s"))
                .arg("git")
                .arg("-C")
                .arg(self.repo)
                .arg("-c")
                .arg("core.hooksPath=/dev/null")
                .args(args)
                .env("GIT_OPTIONAL_LOCKS", "0")
                .env("GIT_TERMINAL_PROMPT", "0")
                .env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes");

            let out = cmd
                .output()
                .map_err(|e| format!("timeout git {}: {e}", args.join(" ")))?;

            if out.status.code() == Some(124) {
                return Err(format!("Таймаут {secs}с"));
            }

            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).to_string())
            }
        }

        #[cfg(not(unix))]
        {
            let out = self
                .base_cmd()
                .args(args)
                .output()
                .map_err(|e| format!("git {}: {e}", args.join(" ")))?;

            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).to_string())
            }
        }
    }

    pub fn is_repo(&self) -> bool {
        self.repo.join(".git").is_dir()
    }

    pub fn status_porcelain(&self) -> Result<String, String> {
        self.run(&["status", "--porcelain"])
    }

    pub fn fetch(&self) -> Result<String, String> {
        self.run_timeout(&["fetch", "--quiet"], FETCH_TIMEOUT_SECS)
    }

    pub fn ahead_behind(&self) -> (u32, u32) {
        let has_up = self
            .run(&["rev-parse", "--abbrev-ref", "@{upstream}"])
            .is_ok();

        if !has_up {
            return (0, 0);
        }

        match self.run(&["rev-list", "--left-right", "--count", "HEAD...@{upstream}"]) {
            Ok(out) => {
                let parts: Vec<&str> = out.trim().split_whitespace().collect();
                if parts.len() == 2 {
                    let ahead = parts[0].parse().unwrap_or(0);
                    let behind = parts[1].parse().unwrap_or(0);
                    (ahead, behind)
                } else {
                    (0, 0)
                }
            }
            Err(_) => (0, 0),
        }
    }

    pub fn has_upstream(&self) -> bool {
        self.run(&["rev-parse", "--abbrev-ref", "@{upstream}"])
            .is_ok()
    }

    pub fn first_remote(&self) -> Option<String> {
        self.run(&["remote"])
            .ok()
            .and_then(|out| out.lines().next().map(|s| s.to_owned()))
    }

    pub fn current_branch(&self) -> Option<String> {
        self.run(&["symbolic-ref", "--short", "-q", "HEAD"])
            .ok()
            .map(|s| s.trim().to_owned())
    }

    pub fn remote_branch_exists(&self, remote: &str, branch: &str) -> bool {
        self.run_timeout(
            &["ls-remote", "--exit-code", "--heads", remote, branch],
            15,
        )
        .is_ok()
    }

    pub fn add_all(&self) -> Result<(), String> {
        self.run(&["add", "-A", "--", "."])?;
        Ok(())
    }

    pub fn commit(&self, msg: &str) -> Result<(), String> {
        self.run_timeout(&["commit", "--no-verify", "--quiet", "-m", msg], 15)?;
        Ok(())
    }

    pub fn pull(&self) -> Result<String, String> {
        self.run_timeout(&["pull", "--no-rebase", "--quiet"], GIT_OP_TIMEOUT_SECS)
    }

    pub fn push(&self) -> Result<String, String> {
        self.run_timeout(&["push", "--quiet"], GIT_OP_TIMEOUT_SECS)
    }

    pub fn push_set_upstream(&self, remote: &str) -> Result<String, String> {
        self.run_timeout(
            &["push", "--quiet", "-u", remote, "HEAD"],
            GIT_OP_TIMEOUT_SECS,
        )
    }

    pub fn set_upstream_to(&self, remote: &str, branch: &str) -> Result<(), String> {
        self.run(&["branch", &format!("--set-upstream-to={remote}/{branch}")])?;
        Ok(())
    }

    pub fn conflicted_files(&self) -> Vec<String> {
        match self.run(&["diff", "--name-only", "--diff-filter=U", "-z"]) {
            Ok(out) => out
                .split('\0')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned())
                .collect(),
            Err(_) => vec![],
        }
    }

    pub fn checkout_ours(&self, file: &str) -> Result<(), String> {
        self.run(&["checkout", "--ours", "--", file])?;
        Ok(())
    }

    pub fn add_file(&self, file: &str) -> Result<(), String> {
        self.run(&["add", "--", file])?;
        Ok(())
    }

    pub fn rm_file(&self, file: &str) -> Result<(), String> {
        self.run(&["rm", "-f", "--quiet", "--", file])?;
        Ok(())
    }

    pub fn show_stage(&self, stage: u8, file: &str) -> Option<Vec<u8>> {
        let spec = format!(":{stage}:{file}");
        let out = self
            .base_cmd()
            .args(["show", &spec])
            .output()
            .ok()?;

        if out.status.success() {
            Some(out.stdout)
        } else {
            None
        }
    }

    pub fn cat_file_exists(&self, spec: &str) -> bool {
        self.run(&["cat-file", "-e", spec]).is_ok()
    }

    pub fn merge_abort(&self) {
        let _ = self.run(&["merge", "--abort"]);
    }

    pub fn ensure_identity(&self) -> Result<(), String> {
        let name = self.run(&["config", "user.name"]).unwrap_or_default();
        let email = self.run(&["config", "user.email"]).unwrap_or_default();

        if !name.trim().is_empty() && !email.trim().is_empty() {
            return Ok(());
        }

        let gname = read_global_config("user.name");
        let gemail = read_global_config("user.email");

        if gname.is_empty() || gemail.is_empty() {
            return Err("Git identity not configured".to_owned());
        }

        if name.trim().is_empty() {
            self.run(&["config", "user.name", &gname])?;
        }
        if email.trim().is_empty() {
            self.run(&["config", "user.email", &gemail])?;
        }

        Ok(())
    }

    pub fn log_recent(&self, n: usize) -> Vec<(String, String)> {
        let out = self
            .run(&[
                "log",
                "--all",
                "-n",
                &n.to_string(),
                "--format=%cd|%s",
                "--date=format:%d/%m %H:%M",
            ])
            .unwrap_or_default();

        out.lines()
            .filter_map(|line| {
                let mut parts = line.splitn(2, '|');
                let date = parts.next()?.to_owned();
                let msg = parts.next()?.to_owned();
                Some((date, msg))
            })
            .collect()
    }

    pub fn ensure_safe_directory(&self) {
        let repo_str = self.repo.to_string_lossy();

        let vals = Command::new("git")
            .args(["config", "--global", "--get-all", "safe.directory"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();

        if vals.lines().any(|l| l.trim() == "*") {
            return;
        }

        if !vals.lines().any(|l| l.trim() == repo_str.as_ref()) {
            let _ = Command::new("git")
                .args(["config", "--global", "--add", "safe.directory", &repo_str])
                .output();
        }
    }

    pub fn ensure_conflict_dir_ignored(&self, conflict_dir: &str) {
        let exclude_path = self.repo.join(".git/info/exclude");
        let pattern = format!("/{conflict_dir}/");

        if let Some(parent) = exclude_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let content = std::fs::read_to_string(&exclude_path).unwrap_or_default();
        if !content.lines().any(|l| l == pattern) {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&exclude_path)
                .and_then(|mut f| {
                    use std::io::Write;
                    writeln!(f, "{pattern}")
                });
        }
    }

    pub fn untrack_conflict_dir(&self, conflict_dir: &str) {
        let out = self
            .run(&["ls-files", "--", conflict_dir])
            .unwrap_or_default();

        if !out.trim().is_empty() {
            let _ = self.run(&["rm", "-r", "--cached", "--quiet", "--", conflict_dir]);
        }
    }
}

fn read_global_config(key: &str) -> String {
    match Command::new("git").args(["config", "--global", key]).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_owned(),
        _ => String::new(),
    }
}