use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapProfileReportV1 {
    pub schema_version: u32,
    pub tool: ToolInfo,
    pub dataset: DatasetInfo,
    pub config: RunConfig,
    pub runs: Vec<RunEntry>,
    pub summary: PhaseSummary,
    pub root_validation: RootValidation,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: String,
    pub git_sha: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetInfo {
    pub path: String,
    pub manifest_sha256: String,
    pub chain_id: u64,
    pub pivot_block: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunConfig {
    pub backend: String,
    pub repeat: usize,
    pub warmup: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunEntry {
    pub index: usize,
    pub is_warmup: bool,
    pub insert_accounts_secs: f64,
    pub insert_storages_secs: f64,
    pub total_secs: f64,
    pub state_root: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PhaseSummary {
    pub insert_accounts: PhaseStats,
    pub insert_storages: PhaseStats,
    pub total: PhaseStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PhaseStats {
    pub median_secs: f64,
    pub mean_secs: f64,
    pub stddev_secs: f64,
    pub p95_secs: f64,
    pub p99_secs: f64,
    pub min_secs: f64,
    pub max_secs: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RootValidation {
    pub computed: String,
    pub expected: String,
    pub matches: bool,
}

impl PhaseStats {
    pub fn from_durations(durations: &[Duration]) -> Self {
        let mut sorted: Vec<f64> = durations.iter().map(|d| d.as_secs_f64()).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = sorted.len();
        if n == 0 {
            return Self {
                median_secs: 0.0,
                mean_secs: 0.0,
                stddev_secs: 0.0,
                p95_secs: 0.0,
                p99_secs: 0.0,
                min_secs: 0.0,
                max_secs: 0.0,
            };
        }

        let median = if n % 2 == 1 {
            sorted[n / 2]
        } else {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        };

        let mean = sorted.iter().sum::<f64>() / n as f64;

        let stddev = if n < 2 {
            0.0
        } else {
            let variance =
                sorted.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
            variance.sqrt()
        };

        let percentile = |p: f64| -> f64 {
            let idx = ((p / 100.0) * (n - 1) as f64).round() as usize;
            sorted[idx.min(n - 1)]
        };

        Self {
            median_secs: median,
            mean_secs: mean,
            stddev_secs: stddev,
            p95_secs: percentile(95.0),
            p99_secs: percentile(99.0),
            min_secs: sorted[0],
            max_secs: sorted[n - 1],
        }
    }
}

/// Compute the SHA-256 hash of a file's contents, returned as a lowercase hex string.
pub fn compute_manifest_sha256(manifest_path: &Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};

    let mut file = fs::File::open(manifest_path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

impl SnapProfileReportV1 {
    pub fn write_to_file(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }

    pub fn load_from_file(path: &Path) -> eyre::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let report: Self = serde_json::from_str(&contents)?;
        Ok(report)
    }
}
