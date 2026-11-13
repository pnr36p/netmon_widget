use std::sync::OnceLock;
use std::time::Duration;

fn shared_http_client() -> Result<&'static reqwest::Client, String> {
    static CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();
    let res = CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .danger_accept_invalid_certs(false)
            .pool_max_idle_per_host(0)
            .user_agent(concat!("netmon_widget/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| e.to_string())
    });
    res.as_ref().map_err(|e| e.clone())
}

pub async fn check(host: &str, port: u16, use_https: bool, timeout: Duration) -> Result<(), String> {
    let trimmed = host.trim();
    let url = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        let scheme = if use_https || port == 443 || port == 8443 {
            "https"
        } else {
            "http"
        };
        let is_default_port =
            (scheme == "https" && port == 443) || (scheme == "http" && port == 80) || port == 0;
        if is_default_port {
            format!("{}://{}", scheme, trimmed)
        } else {
            format!("{}://{}:{}", scheme, trimmed, port)
        }
    };

    let client = shared_http_client()?;
    let resp = client
        .get(&url)
        .timeout(timeout)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() || resp.status().is_redirection() {
        Ok(())
    } else {
        Err(format!("HTTP {}", resp.status()))
    }
}
