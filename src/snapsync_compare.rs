use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::snapsync_report::SnapProfileReportV1;

pub struct CompareOptions {
    pub baseline: PathBuf,
    pub candidate: PathBuf,
    pub regression_threshold_pct: Option<f64>,
    pub fail_on_regression: bool,
    pub json_out: Option<PathBuf>,
    pub json_stdout: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub schema_version: u32,
    pub baseline_path: String,
    pub candidate_path: String,
    pub compatible: bool,
    pub deltas: PhaseDeltaSummary,
    pub regression_detected: bool,
    pub threshold_pct: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PhaseDeltaSummary {
    pub total: PhaseDelta,
    pub insert_accounts: PhaseDelta,
    pub insert_storages: PhaseDelta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PhaseDelta {
    pub median_delta_pct: f64,
    pub p95_delta_pct: f64,
}

impl PhaseDelta {
    fn compute(baseline_median: f64, baseline_p95: f64, candidate_median: f64, candidate_p95: f64) -> Self {
        let median_delta_pct = if baseline_median == 0.0 {
            0.0
        } else {
            ((candidate_median - baseline_median) / baseline_median) * 100.0
        };
        let p95_delta_pct = if baseline_p95 == 0.0 {
            0.0
        } else {
            ((candidate_p95 - baseline_p95) / baseline_p95) * 100.0
        };
        Self {
            median_delta_pct,
            p95_delta_pct,
        }
    }
}

pub fn run_compare(opts: CompareOptions) -> eyre::Result<()> {
    let baseline = SnapProfileReportV1::load_from_file(&opts.baseline)?;
    let candidate = SnapProfileReportV1::load_from_file(&opts.candidate)?;

    // Compatibility checks
    if baseline.schema_version != candidate.schema_version {
        return Err(eyre::eyre!(
            "Schema version mismatch: baseline={} candidate={}",
            baseline.schema_version,
            candidate.schema_version
        ));
    }
    if baseline.dataset.manifest_sha256 != candidate.dataset.manifest_sha256 {
        return Err(eyre::eyre!(
            "Dataset mismatch: baseline manifest_sha256={} candidate manifest_sha256={}",
            baseline.dataset.manifest_sha256,
            candidate.dataset.manifest_sha256
        ));
    }
    if baseline.config.backend != candidate.config.backend {
        return Err(eyre::eyre!(
            "Backend mismatch: baseline={} candidate={}",
            baseline.config.backend,
            candidate.config.backend
        ));
    }

    let deltas = PhaseDeltaSummary {
        total: PhaseDelta::compute(
            baseline.summary.total.median_secs,
            baseline.summary.total.p95_secs,
            candidate.summary.total.median_secs,
            candidate.summary.total.p95_secs,
        ),
        insert_accounts: PhaseDelta::compute(
            baseline.summary.insert_accounts.median_secs,
            baseline.summary.insert_accounts.p95_secs,
            candidate.summary.insert_accounts.median_secs,
            candidate.summary.insert_accounts.p95_secs,
        ),
        insert_storages: PhaseDelta::compute(
            baseline.summary.insert_storages.median_secs,
            baseline.summary.insert_storages.p95_secs,
            candidate.summary.insert_storages.median_secs,
            candidate.summary.insert_storages.p95_secs,
        ),
    };

    let regression_detected = opts
        .regression_threshold_pct
        .is_some_and(|threshold| deltas.total.median_delta_pct > threshold);

    let report = ComparisonReport {
        schema_version: 1,
        baseline_path: opts.baseline.display().to_string(),
        candidate_path: opts.candidate.display().to_string(),
        compatible: true,
        deltas,
        regression_detected,
        threshold_pct: opts.regression_threshold_pct,
    };

    // Print formatted table to terminal
    println!("=== Snap Profile Comparison ===");
    println!();
    println!("Baseline:  {}", report.baseline_path);
    println!("Candidate: {}", report.candidate_path);
    println!();
    println!(
        "{:<20} {:>14} {:>14}",
        "Phase", "Median delta%", "P95 delta%"
    );
    println!("{:-<20} {:-<14} {:-<14}", "", "", "");
    print_phase_row("Total", &report.deltas.total);
    print_phase_row("InsertAccounts", &report.deltas.insert_accounts);
    print_phase_row("InsertStorages", &report.deltas.insert_storages);
    println!();

    if let Some(threshold) = report.threshold_pct {
        println!("Regression threshold: {threshold:+.2}%");
    }
    if report.regression_detected {
        println!("REGRESSION DETECTED: total median delta exceeds threshold");
    } else if report.threshold_pct.is_some() {
        println!("No regression detected.");
    }

    // Write JSON to file if requested
    if let Some(json_path) = &opts.json_out {
        let json = serde_json::to_string_pretty(&report)?;
        std::fs::write(json_path, json)?;
        println!();
        println!("Comparison report written to: {}", json_path.display());
    }

    // Print JSON to stdout if requested
    if opts.json_stdout {
        let json = serde_json::to_string_pretty(&report)?;
        println!("{json}");
    }

    if opts.fail_on_regression && report.regression_detected {
        return Err(eyre::eyre!("Regression detected, failing as requested"));
    }

    Ok(())
}

fn print_phase_row(name: &str, delta: &PhaseDelta) {
    println!(
        "{:<20} {:>+13.2}% {:>+13.2}%",
        name, delta.median_delta_pct, delta.p95_delta_pct
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapsync_report::*;
    use std::time::Duration;

    fn make_report(
        total_median: f64,
        total_p95: f64,
        manifest_sha: &str,
        backend: &str,
    ) -> SnapProfileReportV1 {
        let phase = |median: f64, p95: f64| PhaseStats {
            median_secs: median,
            mean_secs: median,
            stddev_secs: 0.0,
            p95_secs: p95,
            p99_secs: p95,
            min_secs: median * 0.9,
            max_secs: p95 * 1.1,
        };
        SnapProfileReportV1 {
            schema_version: 1,
            tool: ToolInfo {
                name: "ethrex-replay".into(),
                version: "0.1.0".into(),
                git_sha: "abc123".into(),
            },
            dataset: DatasetInfo {
                path: "/tmp/test".into(),
                manifest_sha256: manifest_sha.into(),
                chain_id: 1,
                pivot_block: 100,
            },
            config: RunConfig {
                backend: backend.into(),
                repeat: 5,
                warmup: 1,
            },
            runs: vec![],
            summary: PhaseSummary {
                insert_accounts: phase(total_median * 0.1, total_p95 * 0.1),
                insert_storages: phase(total_median * 0.9, total_p95 * 0.9),
                total: phase(total_median, total_p95),
            },
            root_validation: RootValidation {
                computed: "0x1234".into(),
                expected: "0x1234".into(),
                matches: true,
            },
        }
    }

    fn write_report(dir: &std::path::Path, name: &str, report: &SnapProfileReportV1) -> PathBuf {
        let path = dir.join(name);
        report.write_to_file(&path).unwrap();
        path
    }

    #[test]
    fn identical_reports_show_zero_delta() {
        let dir = tempfile::tempdir().unwrap();
        let report = make_report(100.0, 110.0, "sha256abc", "rocksdb");
        let baseline = write_report(dir.path(), "baseline.json", &report);
        let candidate = write_report(dir.path(), "candidate.json", &report);

        run_compare(CompareOptions {
            baseline,
            candidate,
            regression_threshold_pct: None,
            fail_on_regression: false,
            json_out: None,
            json_stdout: false,
        })
        .unwrap();
    }

    #[test]
    fn regression_detected_above_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_report = make_report(100.0, 110.0, "sha256abc", "rocksdb");
        let candidate_report = make_report(120.0, 130.0, "sha256abc", "rocksdb"); // +20%
        let baseline = write_report(dir.path(), "baseline.json", &baseline_report);
        let candidate = write_report(dir.path(), "candidate.json", &candidate_report);

        let result = run_compare(CompareOptions {
            baseline,
            candidate,
            regression_threshold_pct: Some(5.0),
            fail_on_regression: true,
            json_out: None,
            json_stdout: false,
        });

        assert!(result.is_err(), "should fail on regression");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Regression"), "error: {err}");
    }

    #[test]
    fn no_regression_below_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_report = make_report(100.0, 110.0, "sha256abc", "rocksdb");
        let candidate_report = make_report(102.0, 112.0, "sha256abc", "rocksdb"); // +2%
        let baseline = write_report(dir.path(), "baseline.json", &baseline_report);
        let candidate = write_report(dir.path(), "candidate.json", &candidate_report);

        run_compare(CompareOptions {
            baseline,
            candidate,
            regression_threshold_pct: Some(5.0),
            fail_on_regression: true,
            json_out: None,
            json_stdout: false,
        })
        .unwrap();
    }

    #[test]
    fn mismatched_manifest_sha_fails() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_report = make_report(100.0, 110.0, "sha_aaa", "rocksdb");
        let candidate_report = make_report(100.0, 110.0, "sha_bbb", "rocksdb");
        let baseline = write_report(dir.path(), "baseline.json", &baseline_report);
        let candidate = write_report(dir.path(), "candidate.json", &candidate_report);

        let result = run_compare(CompareOptions {
            baseline,
            candidate,
            regression_threshold_pct: None,
            fail_on_regression: false,
            json_out: None,
            json_stdout: false,
        });

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Dataset mismatch"));
    }

    #[test]
    fn mismatched_backend_fails() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_report = make_report(100.0, 110.0, "sha256abc", "rocksdb");
        let candidate_report = make_report(100.0, 110.0, "sha256abc", "inmemory");
        let baseline = write_report(dir.path(), "baseline.json", &baseline_report);
        let candidate = write_report(dir.path(), "candidate.json", &candidate_report);

        let result = run_compare(CompareOptions {
            baseline,
            candidate,
            regression_threshold_pct: None,
            fail_on_regression: false,
            json_out: None,
            json_stdout: false,
        });

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Backend mismatch"));
    }

    #[test]
    fn json_comparison_report_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_report = make_report(100.0, 110.0, "sha256abc", "rocksdb");
        let candidate_report = make_report(105.0, 115.0, "sha256abc", "rocksdb");
        let baseline = write_report(dir.path(), "baseline.json", &baseline_report);
        let candidate = write_report(dir.path(), "candidate.json", &candidate_report);
        let json_out = dir.path().join("comparison.json");

        run_compare(CompareOptions {
            baseline,
            candidate,
            regression_threshold_pct: Some(10.0),
            fail_on_regression: false,
            json_out: Some(json_out.clone()),
            json_stdout: false,
        })
        .unwrap();

        let contents = std::fs::read_to_string(&json_out).unwrap();
        let report: ComparisonReport = serde_json::from_str(&contents).unwrap();
        assert!(report.compatible);
        assert!(!report.regression_detected);
        assert!((report.deltas.total.median_delta_pct - 5.0).abs() < 0.01);
    }

    #[test]
    fn phase_stats_from_durations() {
        let durations = vec![
            Duration::from_secs(10),
            Duration::from_secs(20),
            Duration::from_secs(30),
        ];
        let stats = PhaseStats::from_durations(&durations);
        assert!((stats.median_secs - 20.0).abs() < 0.01);
        assert!((stats.mean_secs - 20.0).abs() < 0.01);
        assert!((stats.min_secs - 10.0).abs() < 0.01);
        assert!((stats.max_secs - 30.0).abs() < 0.01);
    }
}
