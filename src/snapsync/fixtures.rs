use std::path::Path;

use ethrex_common::types::AccountState;
use ethrex_common::{H256, U256};
use ethrex_p2p::sync::profile::{DatasetPaths, PivotInfo, SnapProfileManifest};
use ethrex_rlp::encode::RLPEncode;

/// Generate a tiny valid dataset at `dir` with 3 accounts and 2 storage slots.
///
/// The state root in the manifest is a placeholder (won't match the computed trie
/// root), so `run_once` will report a mismatch. This is intentional — the fixture
/// is for testing dataset loading and RLP validity, not replay correctness.
pub fn generate_tiny_dataset(dir: &Path) -> std::io::Result<()> {
    let acc_dir = dir.join("account_state_snapshots");
    let storage_dir = dir.join("account_storages_snapshots");
    std::fs::create_dir_all(&acc_dir)?;
    std::fs::create_dir_all(&storage_dir)?;

    // 3 accounts with default storage_root (EMPTY_TRIE_HASH) and code_hash (EMPTY_KECCACK_HASH)
    let accounts: Vec<(H256, AccountState)> = vec![
        (
            H256::from_low_u64_be(1),
            AccountState {
                nonce: 1,
                balance: U256::from(1000),
                ..Default::default()
            },
        ),
        (
            H256::from_low_u64_be(2),
            AccountState {
                nonce: 0,
                balance: U256::from(2000),
                ..Default::default()
            },
        ),
        (
            H256::from_low_u64_be(3),
            AccountState {
                nonce: 5,
                balance: U256::from(500),
                ..Default::default()
            },
        ),
    ];

    let mut buf = Vec::new();
    accounts.encode(&mut buf);
    std::fs::write(acc_dir.join("account_state_chunk.rlp.0"), &buf)?;

    // Storage: 1 entry mapping account 0x01 to 2 storage slots
    let storages: Vec<(Vec<H256>, Vec<(H256, U256)>)> = vec![(
        vec![H256::from_low_u64_be(1)],
        vec![
            (H256::from_low_u64_be(100), U256::from(42)),
            (H256::from_low_u64_be(101), U256::from(99)),
        ],
    )];

    let mut buf = Vec::new();
    storages.encode(&mut buf);
    std::fs::write(
        storage_dir.join("account_storages_chunk.rlp.0"),
        &buf,
    )?;

    // Manifest with placeholder state root
    let manifest = SnapProfileManifest {
        version: 1,
        chain_id: 1,
        rocksdb_enabled: false,
        pivot: PivotInfo {
            number: 100,
            hash: H256::from_low_u64_be(999),
            state_root: H256::from_low_u64_be(888),
            timestamp: 1700000000,
        },
        post_accounts_insert_state_root: H256::from_low_u64_be(777),
        paths: DatasetPaths {
            account_state_snapshots_dir: "account_state_snapshots".into(),
            account_storages_snapshots_dir: "account_storages_snapshots".into(),
        },
    };

    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(dir.join("manifest.json"), json)?;

    Ok(())
}

/// Dataset with snapshot dirs but no manifest.json.
pub fn generate_corrupt_missing_manifest(dir: &Path) -> std::io::Result<()> {
    let acc_dir = dir.join("account_state_snapshots");
    let storage_dir = dir.join("account_storages_snapshots");
    std::fs::create_dir_all(&acc_dir)?;
    std::fs::create_dir_all(&storage_dir)?;
    // Write dummy files so dirs are non-empty
    std::fs::write(acc_dir.join("dummy.rlp.0"), b"placeholder")?;
    std::fs::write(storage_dir.join("dummy.rlp.0"), b"placeholder")?;
    Ok(())
}

/// Valid manifest and account chunks, but the storage snapshot dir is empty.
pub fn generate_corrupt_empty_storage_dir(dir: &Path) -> std::io::Result<()> {
    generate_tiny_dataset(dir)?;
    let storage_dir = dir.join("account_storages_snapshots");
    for entry in std::fs::read_dir(&storage_dir)? {
        std::fs::remove_file(entry?.path())?;
    }
    Ok(())
}

/// Valid manifest but account chunk contains garbage bytes instead of RLP.
pub fn generate_corrupt_bad_rlp(dir: &Path) -> std::io::Result<()> {
    generate_tiny_dataset(dir)?;
    let acc_dir = dir.join("account_state_snapshots");
    std::fs::write(
        acc_dir.join("account_state_chunk.rlp.0"),
        b"\xff\xfe\xfd\xfc",
    )?;
    Ok(())
}

/// Valid data but manifest declares an unsupported version (99).
pub fn generate_corrupt_bad_version(dir: &Path) -> std::io::Result<()> {
    generate_tiny_dataset(dir)?;
    let manifest_path = dir.join("manifest.json");
    let contents = std::fs::read_to_string(&manifest_path)?;
    let mut value: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    value["version"] = serde_json::json!(99);
    let json = serde_json::to_string_pretty(&value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(&manifest_path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethrex_p2p::sync::profile::load_manifest;
    use ethrex_rlp::decode::RLPDecode;

    #[test]
    fn tiny_dataset_has_valid_rlp() {
        let dir = tempfile::tempdir().unwrap();
        generate_tiny_dataset(dir.path()).unwrap();

        // Account chunk decodes correctly
        let acc_bytes = std::fs::read(
            dir.path()
                .join("account_state_snapshots/account_state_chunk.rlp.0"),
        )
        .unwrap();
        let accounts: Vec<(H256, AccountState)> =
            RLPDecode::decode(&acc_bytes).expect("account chunk should be valid RLP");
        assert_eq!(accounts.len(), 3);
        assert_eq!(accounts[0].0, H256::from_low_u64_be(1));
        assert_eq!(accounts[0].1.nonce, 1);
        assert_eq!(accounts[0].1.balance, U256::from(1000));

        // Storage chunk decodes correctly
        let storage_bytes = std::fs::read(
            dir.path()
                .join("account_storages_snapshots/account_storages_chunk.rlp.0"),
        )
        .unwrap();
        let storages: Vec<(Vec<H256>, Vec<(H256, U256)>)> =
            RLPDecode::decode(&storage_bytes).expect("storage chunk should be valid RLP");
        assert_eq!(storages.len(), 1);
        assert_eq!(storages[0].0, vec![H256::from_low_u64_be(1)]);
        assert_eq!(storages[0].1.len(), 2);

        // Manifest loads and validates
        let manifest = load_manifest(dir.path()).expect("manifest should load");
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.pivot.number, 100);
    }

    #[test]
    fn corrupt_missing_manifest_has_no_manifest_file() {
        let dir = tempfile::tempdir().unwrap();
        generate_corrupt_missing_manifest(dir.path()).unwrap();
        assert!(!dir.path().join("manifest.json").exists());
        assert!(dir.path().join("account_state_snapshots").exists());
    }

    #[test]
    fn corrupt_empty_storage_dir_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        generate_corrupt_empty_storage_dir(dir.path()).unwrap();
        assert!(dir.path().join("manifest.json").exists());
        let count = std::fs::read_dir(dir.path().join("account_storages_snapshots"))
            .unwrap()
            .count();
        assert_eq!(count, 0);
    }

    #[test]
    fn corrupt_bad_rlp_has_garbage_bytes() {
        let dir = tempfile::tempdir().unwrap();
        generate_corrupt_bad_rlp(dir.path()).unwrap();
        let acc_bytes = std::fs::read(
            dir.path()
                .join("account_state_snapshots/account_state_chunk.rlp.0"),
        )
        .unwrap();
        assert_eq!(acc_bytes, b"\xff\xfe\xfd\xfc");
    }

    #[test]
    fn corrupt_bad_version_has_version_99() {
        let dir = tempfile::tempdir().unwrap();
        generate_corrupt_bad_version(dir.path()).unwrap();
        let contents = std::fs::read_to_string(dir.path().join("manifest.json")).unwrap();
        let value: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(value["version"], 99);
    }

    /// Generate the committed fixture at fixtures/snapsync/v1/tiny/.
    /// Run with: cargo test -- --ignored generate_committed_fixture
    #[test]
    #[ignore]
    fn generate_committed_fixture() {
        let fixture_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/snapsync/v1/tiny");
        if fixture_dir.exists() {
            std::fs::remove_dir_all(&fixture_dir).unwrap();
        }
        generate_tiny_dataset(&fixture_dir).unwrap();
        eprintln!("Fixture written to {}", fixture_dir.display());
    }
}
