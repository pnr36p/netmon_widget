use std::time::Duration;

pub async fn connect(host: &str, port: u16, timeout: Duration) -> Result<(), String> {
    let trimmed = host.trim();
    let addr = if trimmed.contains(':') && !trimmed.starts_with('[') {
        format!("[{}]:{}", trimmed, port)
    } else {
        format!("{}:{}", trimmed, port)
    };
    match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("timeout".to_string()),
    }
}
