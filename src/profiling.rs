use std::fmt;
use std::time::Duration;

pub struct RunStats {
    sorted: Vec<Duration>,
}

impl RunStats {
    pub fn new(mut durations: Vec<Duration>) -> Self {
        durations.sort();
        Self { sorted: durations }
    }

    pub fn min(&self) -> Duration {
        self.sorted[0]
    }

    pub fn max(&self) -> Duration {
        *self.sorted.last().unwrap()
    }

    pub fn mean(&self) -> Duration {
        let total: Duration = self.sorted.iter().sum();
        total / self.sorted.len() as u32
    }

    pub fn median(&self) -> Duration {
        let len = self.sorted.len();
        if len % 2 == 0 {
            (self.sorted[len / 2 - 1] + self.sorted[len / 2]) / 2
        } else {
            self.sorted[len / 2]
        }
    }

    pub fn stddev_ms(&self) -> f64 {
        let mean_ms = self.mean().as_secs_f64() * 1000.0;
        let variance = self
            .sorted
            .iter()
            .map(|d| {
                let diff = d.as_secs_f64() * 1000.0 - mean_ms;
                diff * diff
            })
            .sum::<f64>()
            / self.sorted.len() as f64;
        variance.sqrt()
    }

    pub fn percentile(&self, p: f64) -> Duration {
        let idx = ((p / 100.0) * (self.sorted.len() - 1) as f64).round() as usize;
        self.sorted[idx.min(self.sorted.len() - 1)]
    }

    pub fn len(&self) -> usize {
        self.sorted.len()
    }
}

impl fmt::Display for RunStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  min:    {:>8.2?}", self.min())?;
        writeln!(f, "  max:    {:>8.2?}", self.max())?;
        writeln!(f, "  mean:   {:>8.2?}", self.mean())?;
        writeln!(f, "  median: {:>8.2?}", self.median())?;
        writeln!(f, "  stddev: {:>8.2}ms", self.stddev_ms())?;
        writeln!(f, "  p95:    {:>8.2?}", self.percentile(95.0))?;
        write!(f, "  p99:    {:>8.2?}", self.percentile(99.0))
    }
}

pub fn print_individual_runs(prep: &[Duration], exec: &[Duration]) {
    for (i, (p, e)) in prep.iter().zip(exec.iter()).enumerate() {
        let total = *p + *e;
        tracing::info!(
            "  #{:>2}: prep={:>8.2?} exec={:>8.2?} total={:>8.2?}",
            i + 1, p, e, total
        );
    }
}
