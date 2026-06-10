use crate::histogram::{ErrorCounters, ThreadStats};
use hdrhistogram::Histogram;

pub struct AggregateStats {
    pub latency: Histogram<u64>,
    pub requests: u64,
    pub errors: ErrorCounters,
    pub bytes: u64,
    pub duration_us: u64,
}

pub fn merge(threads: Vec<ThreadStats>) -> AggregateStats {
    let mut agg = AggregateStats {
        latency: Histogram::new_with_bounds(1, 60_000_000, 3).unwrap(),
        requests: 0,
        errors: ErrorCounters::default(),
        bytes: 0,
        duration_us: 0,
    };
    for t in threads {
        let _ = agg.latency.add(&t.latency);
        agg.requests += t.requests;
        agg.errors.merge(&t.errors);
        agg.bytes += t.bytes;
        agg.duration_us = agg.duration_us.max(t.duration_us);
    }
    agg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_two_threads() {
        let mut t1 = ThreadStats::new();
        t1.record_latency(100);
        t1.add_bytes(512);
        t1.duration_us = 1_000_000;

        let mut t2 = ThreadStats::new();
        t2.record_latency(200);
        t2.add_bytes(512);
        t2.duration_us = 1_200_000;

        let agg = merge(vec![t1, t2]);
        assert_eq!(agg.requests, 2);
        assert_eq!(agg.bytes, 1024);
        assert_eq!(agg.duration_us, 1_200_000);
        assert!(agg.latency.value_at_quantile(0.5) >= 100);
    }

    #[test]
    fn empty_merge_gives_zero_stats() {
        let agg = merge(vec![]);
        assert_eq!(agg.requests, 0);
        assert_eq!(agg.bytes, 0);
    }
}
