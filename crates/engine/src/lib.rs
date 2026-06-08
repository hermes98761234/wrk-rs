pub mod config;
pub mod connection;
pub mod thread;
pub use config::BenchConfig;
pub use thread::{spawn_threads, ThreadResult};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    #[test]
    fn benchmark_local_echo_server() {
        // Start a minimal echo server on a random port
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            use std::io::{Read, Write};
            loop {
                if let Ok((mut stream, _)) = listener.accept() {
                    let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: keep-alive\r\n\r\nOK";
                    let mut buf = [0u8; 1024];
                    loop {
                        match stream.read(&mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(_) => {
                                if stream.write_all(response).is_err() { break; }
                            }
                        }
                    }
                }
            }
        });

        // Give server a moment to start
        std::thread::sleep(Duration::from_millis(50));

        let config = BenchConfig {
            threads: 2,
            connections: 4,
            duration: Duration::from_secs(2),
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
        let total_requests: u64 = results.iter().map(|r| r.stats.requests).sum();
        let total_errors: u64 = results.iter().map(|r| r.stats.errors).sum();

        assert!(total_requests > 0, "expected non-zero requests, got 0");
        assert_eq!(total_errors, 0, "expected zero errors, got {}", total_errors);
    }
}
