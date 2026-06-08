pub mod config;
pub mod connection;
pub mod thread;
pub use config::BenchConfig;
pub use thread::{spawn_threads, ThreadResult};
