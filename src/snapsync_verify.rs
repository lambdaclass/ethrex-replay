use std::path::{Path, PathBuf};

use ethrex_common::types::AccountState;
use ethrex_common::{H256, U256};
use ethrex_p2p::sync::profile::load_manifest;
use ethrex_rlp::decode::RLPDecode;
use serde::{Deserialize, Serialize};
use tracing::info;

pub struct VerifyDatasetOptions {
    pub dataset: PathBuf,
    pub strict: bool,
    pub json_out: Option<PathBuf>,
    pub json_stdout: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyResult {
    pub schema_version: u32,
    pub valid: bool,
    pub strict: bool,
    pub errors: Vec<VerifyError>,
    pub stats: DatasetStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyError {
    pub file: String,
    pub message: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DatasetStats {
    pub account_chunks: usize,
    pub storage_chunks: usize,
    pub total_accounts: usize,
    pub total_storage_slots: usize,
}

pub fn run_verify(opts: VerifyDatasetOptions) -> eyre::Result<()> {
    let dataset = &opts.dataset;
    let mut errors = Vec::new();
    let mut stats = DatasetStats::default();

    // 1. Manifest check
    let manifest = match load_manifest(dataset) {
        Ok(m) => Some(m),
        Err(e) => {
            errors.push(VerifyError {
                file: "manifest.json".into(),
                message: format!("Failed to load manifest: {e}"),
            });
            None
        }
    };

    // 2. Version check
    if let Some(ref m) = manifest {
        if m.version != 1 {
            errors.push(VerifyError {
                file: "manifest.json".into(),
                message: format!("Unsupported manifest version: {} (expected 1)", m.version),
            });
        }
    }

    // Resolve directories from manifest (or fall back to default names)
    let acc_dir_name = manifest
        .as_ref()
        .map(|m| m.paths.account_state_snapshots_dir.as_str())
        .unwrap_or("account_state_snapshots");
    let storage_dir_name = manifest
        .as_ref()
        .map(|m| m.paths.account_storages_snapshots_dir.as_str())
        .unwrap_or("account_storages_snapshots");

    let acc_dir = dataset.join(acc_dir_name);
    let storage_dir = dataset.join(storage_dir_name);

    // 3. Required dirs exist and non-empty
    let acc_chunks = check_dir_and_list_chunks(&acc_dir, "account_state_chunk.rlp", &mut errors);
    let storage_chunks =
        check_dir_and_list_chunks(&storage_dir, "account_storages_chunk.rlp", &mut errors);

    stats.account_chunks = acc_chunks.len();
    stats.storage_chunks = storage_chunks.len();

    // 4. Chunk index sanity (unique, contiguous from 0)
    check_chunk_indices(&acc_chunks, acc_dir_name, &mut errors);
    check_chunk_indices(&storage_chunks, storage_dir_name, &mut errors);

    // 5. Strict: decode all chunks
    if opts.strict {
        for chunk_path in &acc_chunks {
            match std::fs::read(chunk_path) {
                Ok(bytes) => {
                    match <Vec<(H256, AccountState)>>::decode(&bytes) {
                        Ok(accounts) => stats.total_accounts += accounts.len(),
                        Err(e) => errors.push(VerifyError {
                            file: chunk_path.display().to_string(),
                            message: format!("Failed to decode account RLP: {e}"),
                        }),
                    }
                }
                Err(e) => errors.push(VerifyError {
                    file: chunk_path.display().to_string(),
                    message: format!("Failed to read file: {e}"),
                }),
            }
        }

        for chunk_path in &storage_chunks {
            match std::fs::read(chunk_path) {
                Ok(bytes) => {
                    match <Vec<(Vec<H256>, Vec<(H256, U256)>)>>::decode(&bytes) {
                        Ok(entries) => {
                            for (_, slots) in &entries {
                                stats.total_storage_slots += slots.len();
                            }
                        }
                        Err(e) => errors.push(VerifyError {
                            file: chunk_path.display().to_string(),
                            message: format!("Failed to decode storage RLP: {e}"),
                        }),
                    }
                }
                Err(e) => errors.push(VerifyError {
                    file: chunk_path.display().to_string(),
                    message: format!("Failed to read file: {e}"),
                }),
            }
        }
    }

    let valid = errors.is_empty();
    let result = VerifyResult {
        schema_version: 1,
        valid,
        strict: opts.strict,
        errors,
        stats,
    };

    // Terminal output
    info!("=== Dataset Verification ===");
    info!("Dataset: {}", dataset.display());
    info!("Strict mode: {}", opts.strict);
    info!("Account chunks: {}", result.stats.account_chunks);
    info!("Storage chunks: {}", result.stats.storage_chunks);
    if opts.strict {
        info!("Total accounts: {}", result.stats.total_accounts);
        info!("Total storage slots: {}", result.stats.total_storage_slots);
    }
    if result.valid {
        info!("Result: VALID");
    } else {
        info!("Result: INVALID ({} errors)", result.errors.len());
        for err in &result.errors {
            info!("  [{}] {}", err.file, err.message);
        }
    }

    // JSON output
    if let Some(json_path) = &opts.json_out {
        let json = serde_json::to_string_pretty(&result)?;
        std::fs::write(json_path, json)?;
        info!("Report written to: {}", json_path.display());
    }
    if opts.json_stdout {
        let json = serde_json::to_string_pretty(&result)?;
        println!("{json}");
    }

    if !result.valid {
        return Err(eyre::eyre!(
            "Dataset verification failed with {} errors",
            result.errors.len()
        ));
    }

    Ok(())
}

/// List all chunk files matching the expected pattern in a directory.
/// Reports errors for missing/empty directories.
fn check_dir_and_list_chunks(
    dir: &Path,
    prefix: &str,
    errors: &mut Vec<VerifyError>,
) -> Vec<PathBuf> {
    if !dir.exists() {
        errors.push(VerifyError {
            file: dir.display().to_string(),
            message: "Directory does not exist".into(),
        });
        return Vec::new();
    }

    let entries: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| name.starts_with(prefix))
            })
            .collect(),
        Err(e) => {
            errors.push(VerifyError {
                file: dir.display().to_string(),
                message: format!("Failed to read directory: {e}"),
            });
            return Vec::new();
        }
    };

    if entries.is_empty() {
        errors.push(VerifyError {
            file: dir.display().to_string(),
            message: "Directory is empty (no matching chunk files)".into(),
        });
    }

    entries
}

/// Run verification and return the result without printing or erroring on failure.
/// This is used by tests to inspect the result directly.
#[cfg(test)]
pub(crate) fn verify_dataset(opts: &VerifyDatasetOptions) -> VerifyResult {
    let dataset = &opts.dataset;
    let mut errors = Vec::new();
    let mut stats = DatasetStats::default();

    let manifest = match load_manifest(dataset) {
        Ok(m) => Some(m),
        Err(e) => {
            errors.push(VerifyError {
                file: "manifest.json".into(),
                message: format!("Failed to load manifest: {e}"),
            });
            None
        }
    };

    if let Some(ref m) = manifest {
        if m.version != 1 {
            errors.push(VerifyError {
                file: "manifest.json".into(),
                message: format!("Unsupported manifest version: {} (expected 1)", m.version),
            });
        }
    }

    let acc_dir_name = manifest
        .as_ref()
        .map(|m| m.paths.account_state_snapshots_dir.as_str())
        .unwrap_or("account_state_snapshots");
    let storage_dir_name = manifest
        .as_ref()
        .map(|m| m.paths.account_storages_snapshots_dir.as_str())
        .unwrap_or("account_storages_snapshots");

    let acc_dir = dataset.join(acc_dir_name);
    let storage_dir = dataset.join(storage_dir_name);

    let acc_chunks = check_dir_and_list_chunks(&acc_dir, "account_state_chunk.rlp", &mut errors);
    let storage_chunks =
        check_dir_and_list_chunks(&storage_dir, "account_storages_chunk.rlp", &mut errors);

    stats.account_chunks = acc_chunks.len();
    stats.storage_chunks = storage_chunks.len();

    check_chunk_indices(&acc_chunks, acc_dir_name, &mut errors);
    check_chunk_indices(&storage_chunks, storage_dir_name, &mut errors);

    if opts.strict {
        for chunk_path in &acc_chunks {
            match std::fs::read(chunk_path) {
                Ok(bytes) => match <Vec<(H256, AccountState)>>::decode(&bytes) {
                    Ok(accounts) => stats.total_accounts += accounts.len(),
                    Err(e) => errors.push(VerifyError {
                        file: chunk_path.display().to_string(),
                        message: format!("Failed to decode account RLP: {e}"),
                    }),
                },
                Err(e) => errors.push(VerifyError {
                    file: chunk_path.display().to_string(),
                    message: format!("Failed to read file: {e}"),
                }),
            }
        }

        for chunk_path in &storage_chunks {
            match std::fs::read(chunk_path) {
                Ok(bytes) => match <Vec<(Vec<H256>, Vec<(H256, U256)>)>>::decode(&bytes) {
                    Ok(entries) => {
                        for (_, slots) in &entries {
                            stats.total_storage_slots += slots.len();
                        }
                    }
                    Err(e) => errors.push(VerifyError {
                        file: chunk_path.display().to_string(),
                        message: format!("Failed to decode storage RLP: {e}"),
                    }),
                },
                Err(e) => errors.push(VerifyError {
                    file: chunk_path.display().to_string(),
                    message: format!("Failed to read file: {e}"),
                }),
            }
        }
    }

    let valid = errors.is_empty();
    VerifyResult {
        schema_version: 1,
        valid,
        strict: opts.strict,
        errors,
        stats,
    }
}

/// Verify chunk indices are unique and contiguous starting from 0.
fn check_chunk_indices(chunks: &[PathBuf], dir_name: &str, errors: &mut Vec<VerifyError>) {
    let mut indices: Vec<usize> = Vec::new();
    for path in chunks {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Pattern: prefix.rlp.<index>
            if let Some(idx_str) = name.rsplit('.').next() {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    indices.push(idx);
                } else {
                    errors.push(VerifyError {
                        file: name.into(),
                        message: format!("Invalid chunk index in filename: {name}"),
                    });
                }
            }
        }
    }

    indices.sort();
    indices.dedup();

    if indices.len() != chunks.len() {
        errors.push(VerifyError {
            file: dir_name.into(),
            message: "Duplicate chunk indices found".into(),
        });
    }

    if !indices.is_empty() && indices != (0..indices.len()).collect::<Vec<_>>() {
        errors.push(VerifyError {
            file: dir_name.into(),
            message: format!(
                "Chunk indices are not contiguous from 0: {:?}",
                indices
            ),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapsync_fixtures::*;

    fn verify(dir: &std::path::Path, strict: bool) -> VerifyResult {
        verify_dataset(&VerifyDatasetOptions {
            dataset: dir.to_path_buf(),
            strict,
            json_out: None,
            json_stdout: false,
        })
    }

    #[test]
    fn valid_tiny_dataset_base_mode() {
        let dir = tempfile::tempdir().unwrap();
        generate_tiny_dataset(dir.path()).unwrap();
        let result = verify(dir.path(), false);
        assert!(result.valid, "errors: {:?}", result.errors);
        assert_eq!(result.stats.account_chunks, 1);
        assert_eq!(result.stats.storage_chunks, 1);
        // Base mode doesn't decode, so counts stay 0
        assert_eq!(result.stats.total_accounts, 0);
        assert_eq!(result.stats.total_storage_slots, 0);
    }

    #[test]
    fn valid_tiny_dataset_strict_mode() {
        let dir = tempfile::tempdir().unwrap();
        generate_tiny_dataset(dir.path()).unwrap();
        let result = verify(dir.path(), true);
        assert!(result.valid, "errors: {:?}", result.errors);
        assert_eq!(result.stats.total_accounts, 3);
        assert_eq!(result.stats.total_storage_slots, 2);
    }

    #[test]
    fn missing_manifest_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        generate_corrupt_missing_manifest(dir.path()).unwrap();
        let result = verify(dir.path(), false);
        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.file == "manifest.json"),
            "should report manifest error"
        );
    }

    #[test]
    fn empty_storage_dir_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        generate_corrupt_empty_storage_dir(dir.path()).unwrap();
        let result = verify(dir.path(), false);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("empty") || e.message.contains("no matching")),
            "should report empty storage dir: {:?}",
            result.errors
        );
    }

    #[test]
    fn bad_rlp_detected_in_strict_mode() {
        let dir = tempfile::tempdir().unwrap();
        generate_corrupt_bad_rlp(dir.path()).unwrap();
        // Base mode: valid (doesn't decode)
        let base = verify(dir.path(), false);
        assert!(base.valid, "base mode should pass: {:?}", base.errors);
        // Strict mode: invalid (garbage bytes fail decode)
        let strict = verify(dir.path(), true);
        assert!(!strict.valid);
        assert!(
            strict
                .errors
                .iter()
                .any(|e| e.message.contains("decode")),
            "should report decode error: {:?}",
            strict.errors
        );
    }

    #[test]
    fn bad_version_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        generate_corrupt_bad_version(dir.path()).unwrap();
        let result = verify(dir.path(), false);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("version")),
            "should report version error: {:?}",
            result.errors
        );
    }

    #[test]
    fn json_output_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        generate_tiny_dataset(dir.path()).unwrap();
        let json_path = dir.path().join("report.json");

        // Run the full flow with json_out
        let _ = run_verify(VerifyDatasetOptions {
            dataset: dir.path().to_path_buf(),
            strict: true,
            json_out: Some(json_path.clone()),
            json_stdout: false,
        });

        // Deserialize and check
        let contents = std::fs::read_to_string(&json_path).unwrap();
        let report: VerifyResult = serde_json::from_str(&contents).unwrap();
        assert!(report.valid);
        assert_eq!(report.stats.total_accounts, 3);
    }

    #[test]
    fn committed_fixture_is_valid() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/snapsync/v1/tiny");
        if !fixture_dir.exists() {
            // Skip if fixture not committed
            return;
        }
        let result = verify(&fixture_dir, true);
        assert!(result.valid, "errors: {:?}", result.errors);
        assert_eq!(result.stats.total_accounts, 3);
        assert_eq!(result.stats.total_storage_slots, 2);
    }
}
