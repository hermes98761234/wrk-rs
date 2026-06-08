mod args;
mod output;

use args::{parse_duration, parse_headers, parse_url, Args};
use clap::Parser;
use wrk_engine::{spawn_threads, BenchConfig};
use wrk_stats::aggregate::merge;

fn main() {
    let cli = Args::parse();

    let (scheme, host, port, path) = parse_url(&cli.url);
    let duration = parse_duration(&cli.duration);
    let timeout = parse_duration(&cli.timeout);
    let headers = parse_headers(&cli.headers);

    let script_source = cli.script.as_ref().map(|path| {
        std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("error: cannot read script {}: {}", path, e);
            std::process::exit(1);
        })
    });

    let config = BenchConfig {
        threads: cli.threads,
        connections: cli.connections,
        duration,
        timeout,
        url: cli.url.clone(),
        scheme,
        host,
        port,
        path,
        method: "GET".to_string(),
        headers,
        body: None,
        script_source,
        script_args: vec![],
        print_latency: cli.latency,
    };

    let results = spawn_threads(config.clone());
    let thread_stats: Vec<_> = results.into_iter().map(|r| r.stats).collect();
    let agg = merge(thread_stats);

    output::print_report(
        &config.url,
        config.threads,
        config.connections,
        config.duration,
        &agg,
        config.print_latency,
    );
}
