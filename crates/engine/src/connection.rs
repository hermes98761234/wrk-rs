use crate::config::BenchConfig;
use mlua::Lua;
use rustls::pki_types::ServerName;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use wrk_http::{parse_response, RequestBuilder};
use wrk_scripting::hooks;
use wrk_stats::ThreadStats;

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

        let addr = format!("{}:{}", config.host, config.port);
        let stream = match timeout(config.timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(s)) => s,
            _ => {
                stats.errors.connect += 1;
                continue;
            }
        };
        // Match wrk: disable Nagle so small requests aren't delayed.
        let _ = stream.set_nodelay(true);

        if let Some(ref connector) = tls_connector {
            let server_name = ServerName::try_from(config.host.clone()).unwrap();
            match timeout(config.timeout, connector.connect(server_name, stream)).await {
                Ok(Ok(mut tls)) => {
                    run_requests(&mut tls, &config, &lua, &default_request, deadline, stats).await
                }
                _ => {
                    stats.errors.connect += 1;
                    continue;
                }
            }
        } else {
            let mut s = stream;
            run_requests(&mut s, &config, &lua, &default_request, deadline, stats).await;
        }
    }
}

/// Issue requests over an established connection (HTTP keep-alive) until the
/// deadline passes, the server closes it, or an error forces a reconnect.
async fn run_requests<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    stream: &mut S,
    config: &BenchConfig,
    lua: &Lua,
    default_request: &[u8],
    deadline: Instant,
    stats: &mut ThreadStats,
) {
    loop {
        if Instant::now() >= deadline {
            return;
        }

        // A failing request() hook is a script bug; stop this connection
        // instead of hot-looping on the error.
        let req = match hooks::call_request(lua, default_request) {
            Ok(r) => r,
            Err(_) => return,
        };

        let start = Instant::now();
        match timeout(config.timeout, stream.write_all(&req)).await {
            Ok(Ok(())) => {}
            Ok(Err(_)) => {
                stats.errors.write += 1;
                return;
            }
            Err(_) => {
                stats.errors.timeout += 1;
                return;
            }
        }

        let keep_alive = match read_response(stream, config.timeout, lua, stats, start).await {
            Some(k) => k,
            None => return,
        };
        if !keep_alive {
            return;
        }

        let delay_ms = hooks::call_delay(lua).unwrap_or(0);
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }
}

/// Read one full HTTP response (headers plus Content-Length body). Records
/// latency, bytes, and status errors. Returns whether the connection may be
/// reused, or None if it must be dropped.
async fn read_response<S: AsyncReadExt + Unpin>(
    stream: &mut S,
    req_timeout: Duration,
    lua: &Lua,
    stats: &mut ThreadStats,
    start: Instant,
) -> Option<bool> {
    let mut buf = Vec::with_capacity(16384);
    let mut chunk = [0u8; 16384];
    let parsed = loop {
        match timeout(req_timeout, stream.read(&mut chunk)).await {
            Ok(Ok(0)) => {
                stats.errors.read += 1;
                return None;
            }
            Ok(Ok(n)) => {
                buf.extend_from_slice(&chunk[..n]);
                if let Some(p) = parse_response(&buf) {
                    let body_len = buf.len() - p.header_len;
                    match p.content_length {
                        Some(len) if body_len < len => continue,
                        _ => break p,
                    }
                }
            }
            Ok(Err(_)) => {
                stats.errors.read += 1;
                return None;
            }
            Err(_) => {
                stats.errors.timeout += 1;
                return None;
            }
        }
    };

    let elapsed_us = start.elapsed().as_micros() as u64;
    stats.record_latency(elapsed_us);
    stats.add_bytes(buf.len() as u64);
    if parsed.status > 399 {
        stats.errors.status += 1;
    }

    let body_len = buf.len() - parsed.header_len;
    let body_end = parsed.header_len + parsed.content_length.unwrap_or(body_len).min(body_len);
    let _ = hooks::call_response(
        lua,
        parsed.status,
        &parsed.headers,
        &buf[parsed.header_len..body_end],
    );

    Some(parsed.keep_alive)
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
