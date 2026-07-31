use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Spinner {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    pub fn start(msg: &str, dim: &str, reset: &str) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        let msg = msg.to_owned();
        let dim = dim.to_owned();
        let reset = reset.to_owned();

        let handle = thread::spawn(move || {
            let mut idx = 0usize;
            while r.load(Ordering::Relaxed) {
                print!(
                    "\r  {}{}{} {}{}{}",
                    dim,
                    FRAMES[idx % FRAMES.len()],
                    reset,
                    dim,
                    msg,
                    reset
                );
                let _ = std::io::stdout().flush();
                idx += 1;
                thread::sleep(Duration::from_millis(70));
            }
        });

        Self {
            running,
            handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        print!("\r\x1b[K");
        let _ = std::io::stdout().flush();
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop();
    }
}