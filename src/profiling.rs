use std::{fmt, time::Duration};

pub struct RunStats {
    durations: Vec<Duration>,
}

impl RunStats {
    pub fn new(mut durations: Vec<Duration>) -> Self {
        durations.sort();
        Self { durations }
    }

    pub fn len(&self) -> usize {
        self.durations.len()
    }

    fn median(&self) -> Duration {
        let n = self.durations.len();
        if n == 0 {
            return Duration::ZERO;
        }
        if n % 2 == 1 {
            self.durations[n / 2]
        } else {
            (self.durations[n / 2 - 1] + self.durations[n / 2]) / 2
        }
    }

    fn mean(&self) -> Duration {
        if self.durations.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.durations.iter().sum();
        total / self.durations.len() as u32
    }

    fn stddev(&self) -> Duration {
        let n = self.durations.len();
        if n < 2 {
            return Duration::ZERO;
        }
        let mean_nanos = self.mean().as_nanos() as f64;
        let variance = self
            .durations
            .iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean_nanos;
                diff * diff
            })
            .sum::<f64>()
            / (n - 1) as f64;
        Duration::from_nanos(variance.sqrt() as u64)
    }

    fn min(&self) -> Duration {
        self.durations.first().copied().unwrap_or(Duration::ZERO)
    }

    fn max(&self) -> Duration {
        self.durations.last().copied().unwrap_or(Duration::ZERO)
    }

    fn percentile(&self, p: f64) -> Duration {
        let n = self.durations.len();
        if n == 0 {
            return Duration::ZERO;
        }
        let idx = ((p / 100.0) * (n - 1) as f64).round() as usize;
        self.durations[idx.min(n - 1)]
    }

    fn p95(&self) -> Duration {
        self.percentile(95.0)
    }

    fn p99(&self) -> Duration {
        self.percentile(99.0)
    }
}

impl fmt::Display for RunStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  median: {:.2?}", self.median())?;
        writeln!(f, "  mean:   {:.2?}", self.mean())?;
        writeln!(f, "  stddev: {:.2?}", self.stddev())?;
        writeln!(f, "  p95:    {:.2?}", self.p95())?;
        writeln!(f, "  p99:    {:.2?}", self.p99())?;
        writeln!(f, "  min:    {:.2?}", self.min())?;
        write!(f, "  max:    {:.2?}", self.max())
    }
}
