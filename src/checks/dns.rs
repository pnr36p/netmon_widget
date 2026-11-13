use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;
use tokio::net::UdpSocket;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::TokioAsyncResolver;

static DNS_TX_ID: AtomicU16 = AtomicU16::new(0x1000);

fn build_dns_a_query(tx_id: u16, domain: &str) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(64);
    pkt.extend_from_slice(&tx_id.to_be_bytes()); // Transaction ID
    pkt.extend_from_slice(&[0x01, 0x00]); // Flags: standard query, recursion desired (RD=1)
    pkt.extend_from_slice(&[0x00, 0x01]); // QDCOUNT = 1
    pkt.extend_from_slice(&[0x00, 0x00]); // ANCOUNT = 0
    pkt.extend_from_slice(&[0x00, 0x00]); // NSCOUNT = 0
    pkt.extend_from_slice(&[0x00, 0x00]); // ARCOUNT = 0

    for label in domain.trim_end_matches('.').split('.') {
        if !label.is_empty() {
            pkt.push(label.len() as u8);
            pkt.extend_from_slice(label.as_bytes());
        }
    }
    pkt.push(0); // Корень QNAME
    pkt.extend_from_slice(&[0x00, 0x01]); // QTYPE = A
    pkt.extend_from_slice(&[0x00, 0x01]); // QCLASS = IN
    pkt
}

async fn query_dns_server(server_ip: IpAddr, port: u16, timeout: Duration) -> Result<(), String> {
    let target = SocketAddr::new(server_ip, if port > 0 { port } else { 53 });
    let bind_addr = if server_ip.is_ipv6() {
        "[::]:0"
    } else {
        "0.0.0.0:0"
    };

    let sock = UdpSocket::bind(bind_addr)
        .await
        .map_err(|e| format!("udp bind: {}", e))?;
    sock.connect(target)
        .await
        .map_err(|e| format!("udp connect: {}", e))?;

    let tx_id = DNS_TX_ID.fetch_add(1, Ordering::Relaxed);
    let query = build_dns_a_query(tx_id, "google.com");

    sock.send(&query)
        .await
        .map_err(|e| format!("udp send: {}", e))?;

    let mut buf = [0u8; 512];
    let n = tokio::time::timeout(timeout, sock.recv(&mut buf))
        .await
        .map_err(|_| "timeout".to_string())?
        .map_err(|e| format!("udp recv: {}", e))?;

    if n < 12 {
        return Err("invalid DNS response (short)".to_string());
    }
    let resp_id = u16::from_be_bytes([buf[0], buf[1]]);
    if resp_id != tx_id {
        return Err("DNS transaction ID mismatch".to_string());
    }
    if (buf[2] & 0x80) == 0 {
        return Err("not a DNS response".to_string());
    }
    let rcode = buf[3] & 0x0F;
    if rcode == 0 || rcode == 3 {
        Ok(())
    } else {
        Err(format!("DNS RCODE {}", rcode))
    }
}

pub async fn resolve(host: &str, port: u16, timeout: Duration) -> Result<(), String> {
    let trimmed = host.trim();

    // Если указан IP-адрес DNS-сервера (например 1.1.1.1, 8.8.8.8, 9.9.9.9),
    // отправляем реальный DNS UDP-запрос и измеряем отклик сервера.
    if let Ok(ip) = trimmed.parse::<IpAddr>() {
        return query_dns_server(ip, port, timeout).await;
    }

    // Если указано доменное имя, резолвим без локального кеша через system/default DNS.
    let (cfg, mut opts) = trust_dns_resolver::system_conf::read_system_conf()
        .unwrap_or_else(|_| (ResolverConfig::default(), ResolverOpts::default()));
    opts.cache_size = 0;
    opts.use_hosts_file = false;
    opts.timeout = timeout;

    let resolver = TokioAsyncResolver::tokio(cfg, opts);
    match tokio::time::timeout(timeout, resolver.lookup_ip(trimmed)).await {
        Ok(Ok(lookup)) => {
            if lookup.iter().next().is_some() {
                Ok(())
            } else {
                Err("no addresses".to_string())
            }
        }
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("timeout".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_valid_dns_query_packet() {
        let pkt = build_dns_a_query(0x1234, "google.com");
        assert_eq!(&pkt[0..4], &[0x12, 0x34, 0x01, 0x00]); // ID + RD
        assert_eq!(&pkt[4..6], &[0x00, 0x01]); // QDCOUNT=1
        // Labels: 6 "google" 3 "com" 0 + QTYPE A(1) + QCLASS IN(1)
        assert_eq!(&pkt[12..], &[6, b'g', b'o', b'o', b'g', b'l', b'e', 3, b'c', b'o', b'm', 0, 0, 1, 0, 1]);
    }
}
