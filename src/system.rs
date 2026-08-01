use std::process::Command;

use crate::ui::Ui;

pub fn version() -> Option<String> {
    let out = Command::new("git").arg("--version").output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    } else {
        None
    }
}

/// Возвращает команду обновления Git для текущей ОС, если определили.
fn upgrade_command() -> Option<(&'static str, Vec<&'static str>)> {
    if cfg!(target_os = "android") {
        return Some(("pkg", vec!["upgrade", "git", "-y"]));
    }
    if cfg!(target_os = "windows") {
        return Some(("winget", vec!["upgrade", "--id", "Git.Git", "-e"]));
    }
    // Linux: автоопределение менеджера
    for (mgr, args) in [
        ("pacman", vec!["-Syu", "--noconfirm", "git"]),
        ("apt", vec!["install", "-y", "--only-upgrade", "git"]),
        ("dnf", vec!["upgrade", "-y", "git"]),
        ("zypper", vec!["update", "-y", "git"]),
        ("apk", vec!["upgrade", "git"]),
    ] {
        if Command::new("which").arg(mgr).output().map(|o| o.status.success()).unwrap_or(false) {
            return Some((mgr, args));
        }
    }
    None
}

pub fn check(ui: &Ui, upgrade: bool) -> Result<(), String> {
    let t = &ui.theme;
    match version() {
        Some(v) => ui.out(&format!(" {}Git: {v}{}", t.green(), t.reset())),
        None => {
            ui.out(&format!(" {}Git не найден в PATH{}", t.red(), t.reset()));
            return Err("git not found".to_owned());
        }
    }

    if !upgrade {
        return Ok(());
    }

    match upgrade_command() {
        Some((mgr, args)) => {
            ui.out(&format!(
                " {}Обновление: {mgr} {}{}",
                t.cyan(),
                args.join(" "),
                t.reset()
            ));
            let status = Command::new(mgr)
                .args(&args)
                .status()
                .map_err(|e| format!("запуск {mgr}: {e}"))?;
            if status.success() {
                ui.out(&format!(" {}Git обновлён{}", t.green(), t.reset()));
                Ok(())
            } else {
                Err(format!("{mgr} завершился с кодом {:?}", status.code()))
            }
        }
        None => {
            ui.out(&format!(
                " {}Не определил пакетный менеджер — обнови Git вручную{}",
                t.yellow(),
                t.reset()
            ));
            Ok(())
        }
    }
}