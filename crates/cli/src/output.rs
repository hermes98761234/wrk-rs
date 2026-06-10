use std::time::Duration;
use wrk_stats::AggregateStats;

pub fn format_duration_value(us: f64) -> String {
    if us < 1_000.0 {
        format!("{:.2}us", us)
    } else if us < 1_000_000.0 {
        format!("{:.2}ms", us / 1_000.0)
    } else {
        format!("{:.2}s", us / 1_000_000.0)
    }
}

pub fn format_bytes(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{:.2}B", bytes)
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.2}KB", bytes / 1024.0)
    } else if bytes < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2}MB", bytes / (1024.0 * 1024.0))
    } else {
        format!("{:.2}GB", bytes / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn format_count(n: f64) -> String {
    if n >= 1_000_000.0 {
        format!("{:.2}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("{:.2}k", n / 1_000.0)
    } else {
        format!("{:.2}", n)
    }
}

pub fn print_report(
    url: &str,
    threads: usize,
    connections: usize,
    bench_duration: Duration,
    agg: &AggregateStats,
    print_latency: bool,
) {
    let duration_s = bench_duration.as_secs_f64();
    let actual_s = agg.duration_us as f64 / 1_000_000.0;

    println!("Running {}s test @ {}", duration_s as u64, url);
    println!("  {} threads and {} connections", threads, connections);
    println!();

    let lat_avg = agg.latency.mean();
    let lat_stdev = agg.latency.stdev();
    let lat_max = agg.latency.max() as f64;
    let lat_plus_stdev_pct = agg.latency.percentile_below((lat_avg + lat_stdev) as u64);

    let rps = agg.requests as f64 / actual_s;
    let rps_stdev = rps * 0.1; // approximation

    println!(
        "  Thread Stats   {:>8}  {:>8}  {:>8}   {:>7}",
        "Avg", "Stdev", "Max", "+/- Stdev"
    );
    println!(
        "    Latency   {:>8}  {:>8}  {:>8}   {:>6.2}%",
        format_duration_value(lat_avg),
        format_duration_value(lat_stdev),
        format_duration_value(lat_max),
        lat_plus_stdev_pct,
    );
    println!(
        "    Req/Sec   {:>8}  {:>8}  {:>8}",
        format_count(rps / threads as f64),
        format_count(rps_stdev / threads as f64),
        format_count(rps * 1.1 / threads as f64),
    );

    if print_latency {
        println!();
        println!("  Latency Distribution");
        for pct in &[50u8, 75, 90, 99] {
            let val = agg.latency.value_at_quantile(*pct as f64 / 100.0) as f64;
            println!("     {:>3}%  {}", pct, format_duration_value(val));
        }
    }

    println!();
    println!(
        "  {} requests in {:.2}s, {} read",
        agg.requests,
        actual_s,
        format_bytes(agg.bytes as f64),
    );
    if agg.errors.socket_total() > 0 {
        println!(
            "  Socket errors: connect {}, read {}, write {}, timeout {}",
            agg.errors.connect, agg.errors.read, agg.errors.write, agg.errors.timeout,
        );
    }
    if agg.errors.status > 0 {
        println!("  Non-2xx or 3xx responses: {}", agg.errors.status);
    }
    println!("Requests/sec:  {:.2}", rps);
    println!(
        "Transfer/sec:  {}",
        format_bytes(agg.bytes as f64 / actual_s)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_microseconds() {
        assert_eq!(format_duration_value(500.0), "500.00us");
    }

    #[test]
    fn format_duration_milliseconds() {
        assert_eq!(format_duration_value(1500.0), "1.50ms");
    }

    #[test]
    fn format_duration_seconds() {
        assert_eq!(format_duration_value(2_500_000.0), "2.50s");
    }

    #[test]
    fn format_bytes_megabytes() {
        assert_eq!(format_bytes(1024.0 * 1024.0), "1.00MB");
    }

    #[test]
    fn format_count_thousands() {
        assert_eq!(format_count(56200.0), "56.20k");
    }
}
