use hdrhistogram::Histogram;

pub struct ThreadStats {
    pub latency: Histogram<u64>,
    pub requests: u64,
    pub errors: u64,
    pub bytes: u64,
    pub duration_us: u64,
}

impl ThreadStats {
    pub fn new() -> Self {
        Self {
            latency: Histogram::new_with_bounds(1, 60_000_000, 3).unwrap(),
            requests: 0,
            errors: 0,
            bytes: 0,
            duration_us: 0,
        }
    }

    pub fn record_latency(&mut self, us: u64) {
        let _ = self.latency.record(us.max(1));
        self.requests += 1;
    }

    pub fn record_error(&mut self) {
        self.errors += 1;
    }

    pub fn add_bytes(&mut self, n: u64) {
        self.bytes += n;
    }
}

impl Default for ThreadStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_latency_and_increments_requests() {
        let mut s = ThreadStats::new();
        s.record_latency(500);
        s.record_latency(1000);
        assert_eq!(s.requests, 2);
        assert_eq!(s.latency.value_at_quantile(0.5), 500);
    }

    #[test]
    fn records_errors_separately() {
        let mut s = ThreadStats::new();
        s.record_error();
        s.record_error();
        assert_eq!(s.errors, 2);
        assert_eq!(s.requests, 0);
    }

    #[test]
    fn accumulates_bytes() {
        let mut s = ThreadStats::new();
        s.add_bytes(1024);
        s.add_bytes(2048);
        assert_eq!(s.bytes, 3072);
    }
}
