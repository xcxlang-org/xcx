pub fn is_safe_url(url_str: &str) -> Result<(), String> {
    if url_str.starts_with("file://") {
        return Err("HALT.FATAL: SSRF - file:// URLs are forbidden".to_string());
    }
    let host = if let Some(start) = url_str.find("://") {
        let remainder = &url_str[start+3..];
        let end = remainder.find('/').unwrap_or(remainder.len());
        let mut host_port = &remainder[..end];
        if let Some(p) = host_port.find('@') { host_port = &host_port[p+1..]; }
        if let Some(p) = host_port.find(':') { host_port = &host_port[..p]; }
        host_port.to_lowercase()
    } else {
        url_str.to_lowercase()
    };
    if host == "169.254.169.254" || host.starts_with("169.254.") {
        return Err("HALT.FATAL: SSRF - Link-local addresses are forbidden".to_string());
    }
    let is_localhost = host == "localhost" || host == "127.0.0.1" || host == "::1";
    if !is_localhost {
        if host.starts_with("10.") ||
            host.starts_with("192.168.") ||
            host.starts_with("172.16.") || host.starts_with("172.17.") ||
            host.starts_with("172.18.") || host.starts_with("172.19.") ||
            host.starts_with("172.20.") || host.starts_with("172.21.") ||
            host.starts_with("172.22.") || host.starts_with("172.23.") ||
            host.starts_with("172.24.") || host.starts_with("172.25.") ||
            host.starts_with("172.26.") || host.starts_with("172.27.") ||
            host.starts_with("172.28.") || host.starts_with("172.29.") ||
            host.starts_with("172.30.") || host.starts_with("172.31.") {
            return Err("HALT.ERROR: SSRF - Private IP ranges are blocked in production".to_string());
        }
    }
    Ok(())
}
