use std::time::Duration;

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

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(false)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if resp.status().is_success() || resp.status().is_redirection() {
        Ok(())
    } else {
        Err(format!("HTTP {}", resp.status()))
    }
}
