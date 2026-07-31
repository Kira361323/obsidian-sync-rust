use crate::config::CONNECTIVITY_TIMEOUT_SECS;
use std::time::Duration;

pub fn is_offline() -> bool {
    let endpoints = [
        "https://connectivitycheck.gstatic.com/generate_204",
        "https://github.com",
    ];

    for ep in &endpoints {
        if check_endpoint(ep) {
            return false;
        }
    }
    true
}

fn check_endpoint(url: &str) -> bool {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(CONNECTIVITY_TIMEOUT_SECS))
        .build();

    agent.get(url).call().is_ok()
}