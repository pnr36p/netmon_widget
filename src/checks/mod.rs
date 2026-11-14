pub mod dns;
pub mod https;
pub mod ping;
pub mod tcp;

use crate::config::CheckConfig;
use crate::state::CheckResult;
use std::time::Instant;

pub async fn run_check(cfg: &CheckConfig) -> CheckResult {
    let start = Instant::now();
    let kind = cfg.base_kind();
    let host = cfg.host.clone();
    let port = cfg.effective_port();
    let timeout = std::time::Duration::from_millis(cfg.timeout_ms);

    let (outcome, latency): (Result<(), String>, u64) = match kind {
        "ping" => match ping::ping(&host, timeout).await {
            Ok(ms) => (Ok(()), ms),
            Err(e) => (Err(e), 0),
        },
        _ => {
            let outcome = match kind {
                "https" => {
                    https::check(
                        &host,
                        port,
                        true,
                        timeout,
                        cfg.expect_status,
                        cfg.expect_text.as_deref(),
                    )
                    .await
                }
                "http" => {
                    https::check(
                        &host,
                        port,
                        false,
                        timeout,
                        cfg.expect_status,
                        cfg.expect_text.as_deref(),
                    )
                    .await
                }
                "dns" => dns::resolve(&host, port, timeout).await,
                "tcp" => tcp::connect(&host, port, timeout).await,
                other => Err(format!("unknown check type: {}", other)),
            };
            let ms = ((start.elapsed().as_micros() + 500) / 1000) as u64;
            (outcome, ms)
        }
    };

    match outcome {
        Ok(()) => CheckResult {
            ok: true,
            latency_ms: Some(latency),
            error: None,
            timestamp: chrono::Local::now(),
        },
        Err(e) => CheckResult {
            ok: false,
            latency_ms: None,
            error: Some(e),
            timestamp: chrono::Local::now(),
        },
    }
}
