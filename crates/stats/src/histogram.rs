use hdrhistogram::Histogram;

/// Error counters matching wrk's categories: socket errors
/// (connect/read/write/timeout) plus non-2xx/3xx HTTP responses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ErrorCounters {
    pub connect: u64,
    pub read: u64,
    pub write: u64,
    pub timeout: u64,
    pub status: u64,
}

impl ErrorCounters {
    pub fn socket_total(&self) -> u64 {
        self.connect + self.read + self.write + self.timeout
    }

    pub fn merge(&mut self, other: &ErrorCounters) {
        self.connect += other.connect;
        self.read += other.read;
        self.write += other.write;
        self.timeout += other.timeout;
        self.status += other.status;
    }
}

pub struct ThreadStats {
    pub latency: Histogram<u64>,
    pub requests: u64,
    pub errors: ErrorCounters,
    pub bytes: u64,
    pub duration_us: u64,
}

impl ThreadStats {
    pub fn new() -> Self {
        Self {
            latency: Histogram::new_with_bounds(1, 60_000_000, 3).unwrap(),
            requests: 0,
            errors: ErrorCounters::default(),
            bytes: 0,
            duration_us: 0,
        }
    }

    pub fn record_latency(&mut self, us: u64) {
        let _ = self.latency.record(us.max(1));
        self.requests += 1;
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
    fn records_errors_by_category() {
        let mut s = ThreadStats::new();
        s.errors.connect += 1;
        s.errors.timeout += 2;
        s.errors.status += 3;
        assert_eq!(s.errors.socket_total(), 3);
        assert_eq!(s.errors.status, 3);
        assert_eq!(s.requests, 0);
    }

    #[test]
    fn merges_error_counters() {
        let mut a = ErrorCounters {
            connect: 1,
            read: 2,
            write: 3,
            timeout: 4,
            status: 5,
        };
        let b = ErrorCounters {
            connect: 10,
            read: 20,
            write: 30,
            timeout: 40,
            status: 50,
        };
        a.merge(&b);
        assert_eq!(a.connect, 11);
        assert_eq!(a.read, 22);
        assert_eq!(a.write, 33);
        assert_eq!(a.timeout, 44);
        assert_eq!(a.status, 55);
        assert_eq!(a.socket_total(), 110);
    }

    #[test]
    fn accumulates_bytes() {
        let mut s = ThreadStats::new();
        s.add_bytes(1024);
        s.add_bytes(2048);
        assert_eq!(s.bytes, 3072);
    }
}
