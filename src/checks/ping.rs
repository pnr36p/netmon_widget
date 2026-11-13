use std::net::{IpAddr, Ipv4Addr};
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
#[repr(C)]
struct IpOptionInformation {
    ttl: u8,
    tos: u8,
    flags: u8,
    options_size: u8,
    options_data: *mut u8,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct IcmpEchoReply {
    address: u32,
    status: u32,
    round_trip_time: u32,
    data_size: u16,
    reserved: u16,
    data: *mut std::ffi::c_void,
    options: IpOptionInformation,
}

#[cfg(target_os = "windows")]
type Handle = *mut std::ffi::c_void;

#[cfg(target_os = "windows")]
#[link(name = "iphlpapi")]
extern "system" {
    fn IcmpCreateFile() -> Handle;
    fn IcmpCloseHandle(handle: Handle) -> i32;
    fn IcmpSendEcho(
        icmp_handle: Handle,
        destination_address: u32,
        request_data: *const u8,
        request_size: u16,
        request_options: *const IpOptionInformation,
        reply_buffer: *mut u8,
        reply_size: u32,
        timeout: u32,
    ) -> u32;
}

fn clean_host(host: &str) -> &str {
    let mut h = host.trim();
    if let Some(rest) = h.strip_prefix("https://").or_else(|| h.strip_prefix("http://")) {
        h = rest;
    }
    if let Some((before_slash, _)) = h.split_once('/') {
        h = before_slash;
    }
    if let Some(stripped) = h.strip_prefix('[') {
        if let Some((inside, _)) = stripped.split_once(']') {
            return inside;
        }
    }
    if h.matches(':').count() == 1 {
        if let Some((host_only, port_part)) = h.split_once(':') {
            if !port_part.is_empty() && port_part.chars().all(|c| c.is_ascii_digit()) {
                return host_only;
            }
        }
    }
    h
}

async fn resolve_ipv4(host: &str, timeout: Duration) -> Option<Ipv4Addr> {
    let clean = clean_host(host);
    if clean.is_empty() {
        return None;
    }
    if let Ok(ip) = clean.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(v4) => Some(v4),
            IpAddr::V6(_) => None,
        };
    }
    if let Ok(Ok(addrs)) = tokio::time::timeout(timeout, tokio::net::lookup_host((clean, 0))).await
    {
        for sa in addrs {
            if let IpAddr::V4(v4) = sa.ip() {
                return Some(v4);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub async fn ping(host: &str, timeout: Duration) -> Result<u64, String> {
    let ip = resolve_ipv4(host, timeout)
        .await
        .ok_or_else(|| "cannot resolve IPv4".to_string())?;
    let timeout_ms = timeout.as_millis().clamp(1, u32::MAX as u128) as u32;
    tokio::task::spawn_blocking(move || ping_windows(ip, timeout_ms))
        .await
        .map_err(|e| format!("join error: {}", e))?
}

#[cfg(target_os = "windows")]
fn ping_windows(ip: Ipv4Addr, timeout_ms: u32) -> Result<u64, String> {
    unsafe {
        let handle = IcmpCreateFile();
        if handle.is_null() {
            return Err("IcmpCreateFile failed".to_string());
        }

        let request_data: [u8; 32] = [0xAB; 32];
        let mut reply_buf = vec![0u8; std::mem::size_of::<IcmpEchoReply>() + request_data.len() + 8];

        let start = Instant::now();
        let ret = IcmpSendEcho(
            handle,
            u32::from(ip).to_be(),
            request_data.as_ptr(),
            request_data.len() as u16,
            std::ptr::null(),
            reply_buf.as_mut_ptr(),
            reply_buf.len() as u32,
            timeout_ms,
        );
        let elapsed_ms = ((start.elapsed().as_micros() + 500) / 1000) as u64;

        IcmpCloseHandle(handle);

        if ret == 0 {
            return Err("no reply / timeout".to_string());
        }

        let reply = &*(reply_buf.as_ptr() as *const IcmpEchoReply);
        // IP_SUCCESS == 0
        if reply.status != 0 {
            return Err(format!("icmp status: {}", reply.status));
        }
        Ok(elapsed_ms)
    }
}

#[cfg(not(target_os = "windows"))]
pub async fn ping(host: &str, timeout: Duration) -> Result<u64, String> {
    use tokio::net::TcpStream;
    use tokio::time::timeout as tokio_timeout;
    let clean = clean_host(host);
    let start = Instant::now();
    for port in [443u16, 80u16] {
        let addr = format!("{}:{}", clean, port);
        if let Ok(Ok(_)) = tokio_timeout(timeout, TcpStream::connect(&addr)).await {
            return Ok(((start.elapsed().as_micros() + 500) / 1000) as u64);
        }
    }
    Err("host unreachable (tcp fallback)".to_string())
}
