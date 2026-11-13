use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
use std::time::Duration;

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

fn resolve_ipv4(host: &str) -> Option<Ipv4Addr> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(v4) => Some(v4),
            _ => None,
        };
    }
    let addr = format!("{}:0", host);
    if let Ok(mut iter) = addr.to_socket_addrs() {
        while let Some(sa) = iter.next() {
            if let IpAddr::V4(v4) = sa.ip() {
                return Some(v4);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub async fn ping(host: &str, timeout: Duration) -> Result<(), String> {
    let host = host.to_string();
    let timeout_ms = timeout.as_millis() as u32;
    tokio::task::spawn_blocking(move || ping_windows(&host, timeout_ms))
        .await
        .map_err(|e| format!("join error: {}", e))?
}

#[cfg(target_os = "windows")]
fn ping_windows(host: &str, timeout_ms: u32) -> Result<(), String> {
    let ip = resolve_ipv4(host).ok_or_else(|| "cannot resolve IPv4".to_string())?;

    unsafe {
        let handle = IcmpCreateFile();
        if handle.is_null() {
            return Err("IcmpCreateFile failed".to_string());
        }

        let request_data: [u8; 32] = [0xAB; 32];
        let mut reply_buf = vec![0u8; std::mem::size_of::<IcmpEchoReply>() + request_data.len() + 8];

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

        IcmpCloseHandle(handle);

        if ret == 0 {
            return Err("no reply / timeout".to_string());
        }

        let reply = &*(reply_buf.as_ptr() as *const IcmpEchoReply);
        // IP_SUCCESS == 0
        if reply.status != 0 {
            return Err(format!("icmp status: {}", reply.status));
        }
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub async fn ping(host: &str, timeout: Duration) -> Result<(), String> {
    use tokio::net::TcpStream;
    use tokio::time::timeout as tokio_timeout;
    for port in [443u16, 80u16] {
        let addr = format!("{}:{}", host, port);
        if let Ok(Ok(_)) = tokio_timeout(timeout, TcpStream::connect(&addr)).await {
            return Ok(());
        }
    }
    Err("host unreachable (tcp fallback)".to_string())
}
