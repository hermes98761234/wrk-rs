use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct BenchConfig {
    pub threads: usize,
    pub connections: usize,
    pub duration: Duration,
    pub timeout: Duration,
    pub url: String,
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub script_source: Option<String>,
    pub script_args: Vec<String>,
    pub print_latency: bool,
}

impl BenchConfig {
    pub fn connections_per_thread(&self) -> usize {
        (self.connections / self.threads).max(1)
    }
}
