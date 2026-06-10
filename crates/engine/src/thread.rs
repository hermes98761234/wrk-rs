use crate::config::BenchConfig;
use crate::connection::{build_default_request, run_connection};
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::Builder;
use wrk_scripting::{hooks, table};
use wrk_stats::ThreadStats;

pub struct ThreadResult {
    pub stats: ThreadStats,
}

pub fn spawn_threads(config: BenchConfig) -> Vec<ThreadResult> {
    let (tx, rx) = std::sync::mpsc::channel();
    let config = Arc::new(config);

    let mut handles = Vec::new();
    for thread_id in 0..config.threads {
        let cfg = Arc::clone(&config);
        let tx = tx.clone();
        let handle = std::thread::spawn(move || {
            let rt = Builder::new_current_thread().enable_all().build().unwrap();
            let stats = rt.block_on(run_thread(thread_id as u32, cfg));
            tx.send(stats).unwrap();
        });
        handles.push(handle);
    }
    drop(tx);

    let results: Vec<ThreadResult> = rx.into_iter().map(|s| ThreadResult { stats: s }).collect();
    for h in handles {
        let _ = h.join();
    }
    results
}

async fn run_thread(thread_id: u32, config: Arc<BenchConfig>) -> ThreadStats {
    let lua = Arc::new(mlua::Lua::new());

    if let Some(src) = &config.script_source {
        lua.load(src).exec().unwrap();
    }
    table::setup_wrk_table(&lua, &config.scheme, &config.host, config.port).unwrap();
    hooks::call_setup(&lua, thread_id).unwrap();
    hooks::call_init(&lua, &config.script_args).unwrap();

    let default_request = Arc::new(build_default_request(&config));
    let conns = config.connections_per_thread();
    let deadline = Instant::now() + config.duration;

    // Build TLS connector if needed
    let tls_connector = if config.scheme == "https" {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        Some(Arc::new(tokio_rustls::TlsConnector::from(Arc::new(
            tls_config,
        ))))
    } else {
        None
    };

    let mut stats = ThreadStats::new();
    let start = Instant::now();

    let mut join_set = tokio::task::JoinSet::new();
    for _ in 0..conns {
        let cfg = Arc::clone(&config);
        let lua = Arc::clone(&lua);
        let req = Arc::clone(&default_request);
        let tls = tls_connector.clone();
        join_set.spawn(async move {
            let mut s = ThreadStats::new();
            run_connection(cfg, lua, deadline, req, &mut s, tls).await;
            s
        });
    }

    while let Some(res) = join_set.join_next().await {
        if let Ok(s) = res {
            stats.requests += s.requests;
            stats.errors.merge(&s.errors);
            stats.bytes += s.bytes;
            let _ = stats.latency.add(&s.latency);
        }
    }
    stats.duration_us = start.elapsed().as_micros() as u64;

    hooks::call_done(&lua, stats.requests, stats.duration_us, stats.bytes).unwrap();
    stats
}
