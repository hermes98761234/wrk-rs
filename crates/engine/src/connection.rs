use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use rustls::pki_types::ServerName;
use wrk_stats::ThreadStats;
use wrk_http::{RequestBuilder, parse_response};
use wrk_scripting::hooks;
use mlua::Lua;
use crate::config::BenchConfig;

pub async fn run_connection(
    config: Arc<BenchConfig>,
    lua: Arc<Lua>,
    deadline: Instant,
    default_request: Arc<Vec<u8>>,
    stats: &mut ThreadStats,
    tls_connector: Option<Arc<TlsConnector>>,
) {
    loop {
        if Instant::now() >= deadline {
            break;
        }
        // Build or script request
        let req = match hooks::call_request(&lua, &default_request) {
            Ok(r) => r,
            Err(_) => { stats.record_error(); continue; }
        };

        // Connect
        let addr = format!("{}:{}", config.host, config.port);
        let stream = match timeout(config.timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(s)) => s,
            _ => { stats.record_error(); continue; }
        };

        let start = Instant::now();

        if let Some(ref connector) = tls_connector {
            let server_name = ServerName::try_from(config.host.clone()).unwrap();
            match timeout(config.timeout, connector.connect(server_name, stream)).await {
                Ok(Ok(mut tls)) => run_request(&mut tls, &req, &lua, config.timeout, stats, start).await,
                _ => { stats.record_error(); continue; }
            }
        } else {
            let mut s = stream;
            run_request(&mut s, &req, &lua, config.timeout, stats, start).await;
        }

        // Delay hook
        let delay_ms = hooks::call_delay(&lua).unwrap_or(0);
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }
}

async fn run_request<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    stream: &mut S,
    req: &[u8],
    lua: &Lua,
    req_timeout: Duration,
    stats: &mut ThreadStats,
    start: Instant,
) {
    if timeout(req_timeout, stream.write_all(req)).await.is_err() {
        stats.record_error();
        return;
    }
    let mut buf = vec![0u8; 16384];
    match timeout(req_timeout, stream.read(&mut buf)).await {
        Ok(Ok(n)) if n > 0 => {
            let elapsed_us = start.elapsed().as_micros() as u64;
            stats.record_latency(elapsed_us);
            stats.add_bytes(n as u64);
            if let Some(parsed) = parse_response(&buf[..n]) {
                let _ = hooks::call_response(lua, parsed.status, &parsed.headers, &buf[parsed.header_len..n]);
            }
        }
        _ => { stats.record_error(); }
    }
}

pub fn build_default_request(config: &BenchConfig) -> Vec<u8> {
    let mut builder = RequestBuilder::new(&config.method, &config.path, &config.host);
    for (k, v) in &config.headers {
        builder = builder.header(k, v);
    }
    if let Some(body) = &config.body {
        builder = builder.body(body.clone());
    }
    builder.build()
}
