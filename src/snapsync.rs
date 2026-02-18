use std::path::PathBuf;

use crate::cli::SnapSyncProfileOptions;
use crate::profiling::RunStats;
use crate::snapsync_report::{
    DatasetInfo, PhaseStats, PhaseSummary, RootValidation, RunConfig, RunEntry,
    SnapProfileReportV1, ToolInfo, compute_manifest_sha256,
};
use ethrex_p2p::sync::profile::load_manifest;
use snapsync_profile::{ProfileBackend, run_once_with_opts};
use tracing::info;

fn parse_backend(name: &str) -> eyre::Result<ProfileBackend> {
    name.parse::<ProfileBackend>()
        .map_err(|e| eyre::eyre!("{e}"))
}

/// Create an isolated DB directory for a single run.
/// Returns (db_dir, guard) where guard is a TempDir that auto-cleans on drop.
#[cfg(feature = "rocksdb")]
fn create_run_db_dir(
    explicit_dir: &Option<PathBuf>,
    run_index: usize,
) -> eyre::Result<(PathBuf, Option<tempfile::TempDir>)> {
    if let Some(base) = explicit_dir {
        let run_dir = base.join(format!("run-{run_index}"));
        std::fs::create_dir_all(&run_dir)?;
        Ok((run_dir, None))
    } else {
        let tmp = tempfile::TempDir::new()?;
        let path = tmp.path().to_path_buf();
        Ok((path, Some(tmp)))
    }
}

pub async fn run_profile(opts: SnapSyncProfileOptions) -> eyre::Result<()> {
    let dataset_path = &opts.dataset;
    let backend = parse_backend(&opts.backend)?;

    // Load and validate manifest
    let manifest =
        load_manifest(dataset_path).map_err(|e| eyre::eyre!("Failed to load dataset: {e}"))?;

    info!("=== SnapSync Offline Profiler ===");
    info!("Dataset: {:?}", dataset_path);
    info!(
        "Pivot block: #{} (hash: {:?})",
        manifest.pivot.number, manifest.pivot.hash
    );
    info!("Backend: {backend}");
    info!("Repeat: {} | Warmup: {}", opts.repeat, opts.warmup);
    info!("");

    let mut insert_accounts_durations = Vec::new();
    let mut insert_storages_durations = Vec::new();
    let mut total_durations = Vec::new();
    let mut last_state_root = None;
    let mut run_entries = Vec::new();

    let total_runs = opts.warmup + opts.repeat;

    for i in 0..total_runs {
        let is_warmup = i < opts.warmup;
        let label = if is_warmup { "warmup" } else { "run" };
        let run_num = if is_warmup {
            i + 1
        } else {
            i - opts.warmup + 1
        };

        // Create a fresh DB directory for each run so timing stats are independent.
        let (db_dir, _temp_dir) = match backend {
            ProfileBackend::InMemory => (PathBuf::from("."), None::<tempfile::TempDir>),
            #[cfg(feature = "rocksdb")]
            ProfileBackend::RocksDb => create_run_db_dir(&opts.db_dir, i)?,
        };

        if !matches!(backend, ProfileBackend::InMemory) {
            info!("[{label} {run_num}] DB dir: {}", db_dir.display());
        }
        info!("[{label} {run_num}] Starting...");

        let result = run_once_with_opts(dataset_path, backend, &db_dir)
            .await
            .map_err(|e| eyre::eyre!("Run failed: {e}"))?;

        // Root consistency check
        if let Some(prev_root) = last_state_root {
            if prev_root != result.computed_state_root {
                return Err(eyre::eyre!(
                    "Non-deterministic state root! Run {} produced {:?}, previous was {:?}",
                    i + 1,
                    result.computed_state_root,
                    prev_root
                ));
            }
        }
        last_state_root = Some(result.computed_state_root);

        info!(
            "[{label} {run_num}] accounts={:.2?} storages={:.2?} total={:.2?}",
            result.insert_accounts_duration, result.insert_storages_duration, result.total_duration,
        );

        run_entries.push(RunEntry {
            index: i,
            is_warmup,
            insert_accounts_secs: result.insert_accounts_duration.as_secs_f64(),
            insert_storages_secs: result.insert_storages_duration.as_secs_f64(),
            total_secs: result.total_duration.as_secs_f64(),
            state_root: format!("{:?}", result.computed_state_root),
        });

        if !is_warmup {
            insert_accounts_durations.push(result.insert_accounts_duration);
            insert_storages_durations.push(result.insert_storages_duration);
            total_durations.push(result.total_duration);
        }

        // Clean up this run's DB unless it's the last measured run and --keep-db is set.
        let is_last_measured = !is_warmup && run_num == opts.repeat;
        #[cfg(feature = "rocksdb")]
        if matches!(backend, ProfileBackend::RocksDb) {
            if is_last_measured && opts.keep_db {
                if let Some(tmp) = _temp_dir {
                    let kept = tmp.keep();
                    info!("DB kept at: {}", kept.display());
                } else {
                    info!("DB kept at: {}", db_dir.display());
                }
            } else if _temp_dir.is_none() && opts.db_dir.is_some() {
                // Explicit --db-dir without --keep-db (or not last run): clean up.
                let _ = std::fs::remove_dir_all(&db_dir);
            }
            // TempDir drops automatically otherwise.
        }
        // Suppress unused variable warning when rocksdb feature is off.
        let _ = is_last_measured;
    }

    // Validate computed state root against expected
    info!("");
    info!("=== Results ({} measured runs) ===", opts.repeat);
    info!("Backend: {backend}");

    let computed_root_str;
    let expected_root_str;
    let root_matches;

    if let Some(root) = last_state_root {
        computed_root_str = format!("{root:?}");
        let expected = manifest.post_accounts_insert_state_root;
        expected_root_str = format!("{expected:?}");
        root_matches = root == expected;

        info!("Computed state root: {computed_root_str}");
        if root_matches {
            info!("Expected state root: {expected_root_str} [MATCH]");
        } else {
            info!("Expected state root: {expected_root_str} [MISMATCH]");
        }
    } else {
        computed_root_str = "none".to_string();
        expected_root_str = format!("{:?}", manifest.post_accounts_insert_state_root);
        root_matches = false;
    }

    info!("");
    if !insert_accounts_durations.is_empty() {
        let stats = RunStats::new(insert_accounts_durations.iter().copied().collect());
        info!("InsertAccounts ({} runs):\n{stats}", stats.len());
    }
    if !insert_storages_durations.is_empty() {
        let stats = RunStats::new(insert_storages_durations.iter().copied().collect());
        info!("InsertStorages ({} runs):\n{stats}", stats.len());
    }
    if !total_durations.is_empty() {
        let stats = RunStats::new(total_durations.iter().copied().collect());
        info!("Total ({} runs):\n{stats}", stats.len());
    }

    // Build JSON report if requested
    if opts.json_out.is_some() || opts.json_stdout {
        let manifest_sha256 = compute_manifest_sha256(&dataset_path.join("manifest.json"))
            .unwrap_or_else(|_| "unknown".to_string());

        let report = SnapProfileReportV1 {
            schema_version: 1,
            tool: ToolInfo {
                name: "ethrex-replay".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                git_sha: option_env!("GIT_SHA").unwrap_or("unknown").to_string(),
            },
            dataset: DatasetInfo {
                path: dataset_path.display().to_string(),
                manifest_sha256,
                chain_id: manifest.chain_id,
                pivot_block: manifest.pivot.number,
            },
            config: RunConfig {
                backend: opts.backend.clone(),
                repeat: opts.repeat,
                warmup: opts.warmup,
            },
            runs: run_entries,
            summary: PhaseSummary {
                insert_accounts: PhaseStats::from_durations(&insert_accounts_durations),
                insert_storages: PhaseStats::from_durations(&insert_storages_durations),
                total: PhaseStats::from_durations(&total_durations),
            },
            root_validation: RootValidation {
                computed: computed_root_str,
                expected: expected_root_str,
                matches: root_matches,
            },
        };

        if let Some(json_path) = &opts.json_out {
            report.write_to_file(json_path)?;
            info!("JSON report written to: {}", json_path.display());
        }
        if opts.json_stdout {
            let json = serde_json::to_string_pretty(&report)
                .map_err(|e| eyre::eyre!("Failed to serialize report: {e}"))?;
            println!("{json}");
        }
    }

    if !root_matches && last_state_root.is_some() {
        return Err(eyre::eyre!(
            "State root mismatch! Computed {} but manifest expects {}",
            last_state_root.map(|r| format!("{r:?}")).unwrap_or_default(),
            manifest.post_accounts_insert_state_root
        ));
    }

    Ok(())
}
