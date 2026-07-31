use crate::config::LOG_MAX_BYTES;
use std::fs;
use std::io::Write;
use std::path::Path;

pub struct Logger {
    path: std::path::PathBuf,
}

impl Logger {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }

    pub fn prepare(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = fs::create_dir_all(dir);
        }

        if let Ok(meta) = fs::metadata(&self.path) {
            if meta.len() > LOG_MAX_BYTES {
                let rotated = self.path.with_extension("log.1");
                let _ = fs::rename(&self.path, rotated);
            }
        }
    }

    pub fn log(&self, msg: &str) {
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!("[{ts}] {msg}\n");

        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

pub fn log_path_exists(path: &Path) -> bool {
    path.exists()
}