use std::fs;
use std::path::Path;

pub fn write(
    path: &Path,
    ok: usize,
    warn: usize,
    err: usize,
    total: usize,
    duration: u64,
) {
    let ts = chrono::Utc::now().timestamp();
    let content = format!(
        "last_sync={ts},ok={ok},warn={warn},err={err},total={total},duration={duration}\n"
    );

    let tmp = path.with_extension("tmp");
    if fs::write(&tmp, &content).is_ok() {
        let _ = fs::rename(&tmp, path);
    }
}