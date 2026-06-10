use clap::{ArgAction, Parser};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "wrk",
    about = "HTTP benchmarking tool",
    version,
    disable_version_flag = true
)]
pub struct Args {
    /// Number of threads to use
    #[arg(short = 't', long = "threads", default_value = "2", value_parser = parse_count)]
    pub threads: usize,

    /// Connections to keep open
    #[arg(short = 'c', long = "connections", default_value = "10", value_parser = parse_count)]
    pub connections: usize,

    /// Duration of test (e.g. 30s, 1m, 2h)
    #[arg(short = 'd', long = "duration", default_value = "10s")]
    pub duration: String,

    /// Load Lua script file
    #[arg(short = 's', long = "script")]
    pub script: Option<String>,

    /// Add header to request (repeatable, format: "Name: Value")
    #[arg(short = 'H', long = "header")]
    pub headers: Vec<String>,

    /// Print latency statistics
    #[arg(long)]
    pub latency: bool,

    /// Socket/request timeout (e.g. 2s)
    #[arg(long, default_value = "2s")]
    pub timeout: String,

    /// Print version details
    #[arg(short = 'v', long = "version", action = ArgAction::Version, value_parser = clap::value_parser!(bool))]
    pub version: Option<bool>,

    /// Target URL
    pub url: String,
}

/// Parse a count that may carry an SI suffix (1k, 1M, 1G), matching wrk.
pub fn parse_count(s: &str) -> Result<usize, String> {
    let (num, mult) = match s.as_bytes().last() {
        Some(b'k') | Some(b'K') => (&s[..s.len() - 1], 1_000usize),
        Some(b'M') => (&s[..s.len() - 1], 1_000_000),
        Some(b'G') => (&s[..s.len() - 1], 1_000_000_000),
        _ => (s, 1),
    };
    num.parse::<usize>()
        .map(|n| n * mult)
        .map_err(|_| format!("invalid count: {s}"))
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

    #[test]
    fn parse_count_plain_and_si_units() {
        assert_eq!(parse_count("400").unwrap(), 400);
        assert_eq!(parse_count("1k").unwrap(), 1_000);
        assert_eq!(parse_count("2M").unwrap(), 2_000_000);
        assert_eq!(parse_count("1G").unwrap(), 1_000_000_000);
        assert!(parse_count("abc").is_err());
    }

    #[test]
    fn args_accept_long_flags() {
        let args = Args::try_parse_from([
            "wrk",
            "--threads",
            "4",
            "--connections",
            "1k",
            "--duration",
            "30s",
            "--header",
            "X-Test: 1",
            "http://localhost/",
        ])
        .unwrap();
        assert_eq!(args.threads, 4);
        assert_eq!(args.connections, 1000);
        assert_eq!(args.duration, "30s");
        assert_eq!(args.headers, vec!["X-Test: 1".to_string()]);
    }
}
