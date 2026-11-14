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

pub fn build_url(host: &str, port: u16, use_https: bool) -> String {
    let trimmed = host.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
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
    }
}

pub async fn check(
    host: &str,
    port: u16,
    use_https: bool,
    timeout: Duration,
    expect_status: Option<u16>,
    expect_text: Option<&str>,
) -> Result<(), String> {
    let url = build_url(host, port, use_https);

    let client = shared_http_client()?;
    let resp = client
        .get(&url)
        .timeout(timeout)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    let status_ok = match expect_status {
        Some(code) => status.as_u16() == code,
        None => status.is_success() || status.is_redirection(),
    };
    if !status_ok {
        return Err(format!("HTTP {}", status));
    }

    if let Some(needle) = expect_text.filter(|s| !s.is_empty()) {
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        let slice = &bytes[..bytes.len().min(512 * 1024)];
        let body = String::from_utf8_lossy(slice);
        if !body.contains(needle) {
            return Err(format!("Missing text: {}", needle));
        }
    }

    Ok(())
}
