use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::time::Duration;
use wrk_engine::{spawn_threads, BenchConfig};
use wrk_stats::aggregate::merge;

fn start_echo_server() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        loop {
            if let Ok((mut stream, _)) = listener.accept() {
                // One thread per connection so keep-alive clients don't
                // starve each other.
                std::thread::spawn(move || {
                    use std::io::{Read, Write};
                    let response =
                        b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: keep-alive\r\n\r\nhello";
                    let mut buf = [0u8; 4096];
                    loop {
                        match stream.read(&mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(_) => {
                                if stream.write_all(response).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                });
            }
        }
    });
    std::thread::sleep(Duration::from_millis(50));
    port
}

fn bench_throughput(c: &mut Criterion) {
    let port = start_echo_server();

    let mut group = c.benchmark_group("engine");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(8));

    group.bench_function("2t_10c_5s", |b| {
        b.iter(|| {
            let config = BenchConfig {
                threads: 2,
                connections: 10,
                duration: Duration::from_secs(5),
                timeout: Duration::from_secs(5),
                url: format!("http://127.0.0.1:{}/", port),
                scheme: "http".to_string(),
                host: "127.0.0.1".to_string(),
                port,
                path: "/".to_string(),
                method: "GET".to_string(),
                headers: HashMap::new(),
                body: None,
                script_source: None,
                script_args: vec![],
                print_latency: false,
            };
            let results = spawn_threads(config);
            let thread_stats: Vec<_> = results.into_iter().map(|r| r.stats).collect();
            let agg = merge(thread_stats);
            let req_per_sec = agg.requests as f64 / (agg.duration_us as f64 / 1_000_000.0);
            criterion::black_box(req_per_sec)
        });
    });

    group.finish();
}

criterion_group!(benches, bench_throughput);
criterion_main!(benches);
