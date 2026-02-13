use crate::cli::SnapSyncProfileOptions;
use crate::profiling::RunStats;
use ethrex_p2p::sync::profile::{load_manifest, run_once};
use tracing::info;

pub async fn run_profile(opts: SnapSyncProfileOptions) -> eyre::Result<()> {
    let dataset_path = &opts.dataset;

    // Load and validate manifest
    let manifest =
        load_manifest(dataset_path).map_err(|e| eyre::eyre!("Failed to load dataset: {e}"))?;

    info!("=== SnapSync Offline Profiler ===");
    info!("Dataset: {:?}", dataset_path);
    info!(
        "Pivot block: #{} (hash: {:?})",
        manifest.pivot.number, manifest.pivot.hash
    );
    info!("Repeat: {} | Warmup: {}", opts.repeat, opts.warmup);
    info!("");

    let mut insert_accounts_durations = Vec::new();
    let mut insert_storages_durations = Vec::new();
    let mut total_durations = Vec::new();
    let mut last_state_root = None;

    let total_runs = opts.warmup + opts.repeat;

    for i in 0..total_runs {
        let is_warmup = i < opts.warmup;
        let label = if is_warmup { "warmup" } else { "run" };
        let run_num = if is_warmup {
            i + 1
        } else {
            i - opts.warmup + 1
        };

        info!("[{label} {run_num}] Starting...");

        let result = run_once(dataset_path)
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

        if !is_warmup {
            insert_accounts_durations.push(result.insert_accounts_duration);
            insert_storages_durations.push(result.insert_storages_duration);
            total_durations.push(result.total_duration);
        }
    }

    // Validate computed state root against expected
    info!("");
    info!("=== Results ({} measured runs) ===", opts.repeat);
    if let Some(root) = last_state_root {
        info!("Computed state root: {:?}", root);
        let expected = manifest.post_accounts_insert_state_root;
        if root != expected {
            return Err(eyre::eyre!(
                "State root mismatch! Computed {:?} but manifest expects {:?}",
                root,
                expected
            ));
        }
        info!("Expected state root: {:?} [MATCH]", expected);
    }

    info!("");
    if !insert_accounts_durations.is_empty() {
        let stats = RunStats::new(insert_accounts_durations);
        info!("InsertAccounts ({} runs):\n{stats}", stats.len());
    }
    if !insert_storages_durations.is_empty() {
        let stats = RunStats::new(insert_storages_durations);
        info!("InsertStorages ({} runs):\n{stats}", stats.len());
    }
    if !total_durations.is_empty() {
        let stats = RunStats::new(total_durations);
        info!("Total ({} runs):\n{stats}", stats.len());
    }

    Ok(())
}
