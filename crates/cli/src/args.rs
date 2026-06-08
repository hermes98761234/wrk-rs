use clap::Parser;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "wrk", about = "HTTP benchmarking tool")]
pub struct Args {
    /// Number of threads
    #[arg(short = 't', default_value = "2")]
    pub threads: usize,

    /// Number of connections
    #[arg(short = 'c', default_value = "10")]
    pub connections: usize,

    /// Duration (e.g. 30s, 1m, 2h)
    #[arg(short = 'd', default_value = "10s")]
    pub duration: String,

    /// Lua script path
    #[arg(short = 's')]
    pub script: Option<String>,

    /// Add header (repeatable, format: "Name: Value")
    #[arg(short = 'H')]
    pub headers: Vec<String>,

    /// Print latency distribution
    #[arg(long)]
    pub latency: bool,

    /// Timeout (e.g. 2s)
    #[arg(long, default_value = "2s")]
    pub timeout: String,

    /// Target URL
    pub url: String,
}

pub fn parse_duration(s: &str) -> Duration {
    if let Some(n) = s.strip_suffix("ms") {
        return Duration::from_millis(n.parse().unwrap_or(10000));
    }
    if let Some(n) = s.strip_suffix('s') {
        return Duration::from_secs(n.parse().unwrap_or(10));
    }
    if let Some(n) = s.strip_suffix('m') {
        return Duration::from_secs(n.parse::<u64>().unwrap_or(1) * 60);
    }
    if let Some(n) = s.strip_suffix('h') {
        return Duration::from_secs(n.parse::<u64>().unwrap_or(1) * 3600);
    }
    Duration::from_secs(s.parse().unwrap_or(10))
}

pub fn parse_url(url: &str) -> (String, String, u16, String) {
    let (scheme, rest) = if let Some(r) = url.strip_prefix("https://") {
        ("https".to_string(), r)
    } else if let Some(r) = url.strip_prefix("http://") {
        ("http".to_string(), r)
    } else {
        ("http".to_string(), url)
    };

    let default_port: u16 = if scheme == "https" { 443 } else { 80 };

    let (host_port, path) = if let Some(idx) = rest.find('/') {
        (&rest[..idx], rest[idx..].to_string())
    } else {
        (rest, "/".to_string())
    };

    let (host, port) = if let Some(idx) = host_port.rfind(':') {
        let h = host_port[..idx].to_string();
        let p = host_port[idx + 1..].parse().unwrap_or(default_port);
        (h, p)
    } else {
        (host_port.to_string(), default_port)
    };

    (scheme, host, port, path)
}

pub fn parse_headers(raw: &[String]) -> HashMap<String, String> {
    raw.iter()
        .filter_map(|h| {
            let mut parts = h.splitn(2, ':');
            let name = parts.next()?.trim().to_string();
            let value = parts.next()?.trim().to_string();
            Some((name, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_seconds() {
        assert_eq!(parse_duration("30s"), Duration::from_secs(30));
    }

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(parse_duration("2m"), Duration::from_secs(120));
    }

    #[test]
    fn parse_url_http_with_path() {
        let (scheme, host, port, path) = parse_url("http://example.com/api");
        assert_eq!(scheme, "http");
        assert_eq!(host, "example.com");
        assert_eq!(port, 80);
        assert_eq!(path, "/api");
    }

    #[test]
    fn parse_url_https_custom_port() {
        let (scheme, _host, port, _path) = parse_url("https://example.com:8443/");
        assert_eq!(scheme, "https");
        assert_eq!(port, 8443);
    }

    #[test]
    fn parse_headers_splits_on_colon() {
        let raw = vec!["Accept: application/json".to_string()];
        let h = parse_headers(&raw);
        assert_eq!(h.get("Accept").unwrap(), "application/json");
    }
}
