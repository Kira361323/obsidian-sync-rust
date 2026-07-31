use std::fs;
use std::path::Path;

pub struct Lock {
    dir: std::path::PathBuf,
}

impl Lock {
    pub fn acquire(dir: std::path::PathBuf) -> Result<Self, String> {
        if let Some(parent) = dir.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if fs::create_dir(&dir).is_ok() {
            let _ = fs::write(dir.join("pid"), std::process::id().to_string());
            return Ok(Self { dir });
        }

        // Проверяем stale lock
        let pid_file = dir.join("pid");
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if process_alive(pid) {
                    return Err(format!("Sync already running (PID {pid})"));
                }
            }
        }

        // Удаляем stale и пробуем снова
        let _ = fs::remove_dir_all(&dir);
        if fs::create_dir(&dir).is_ok() {
            let _ = fs::write(dir.join("pid"), std::process::id().to_string());
            return Ok(Self { dir });
        }

        Err(format!("Cannot acquire lock: {}", dir.display()))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc_kill(pid, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        // Windows: упрощённо — считаем что жив если pid > 0
        pid > 0
    }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "kill"]
    fn libc_kill(pid: u32, sig: i32) -> i32;
}