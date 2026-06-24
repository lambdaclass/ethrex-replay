//! End-to-end tests for L1 custom block generation.

#![cfg(not(feature = "l2"))]

use std::collections::HashSet;

use ethrex_common::{H256, NativeCrypto, types::TxKind};
use ethrex_replay::{
    cache::Cache,
    cli::{CustomBlockOptions, build_custom_l1_cache, load_or_build_custom_l1_cache},
    workloads::Workload,
};

/// Selector of `transfer(address,uint256)`.
const ERC20_TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];

fn options(workload: Workload, n_txs: u64, seed: u64) -> CustomBlockOptions {
    CustomBlockOptions {
        common: Default::default(),
        n_txs: Some(n_txs),
        gas_target: None,
        tx: Some(workload),
        seed,
        n_senders: 8,
        tx_gas: 1_000_000,
        deploy_code_size: 1_024,
        prestate_slots: 1_000,
        save_program_input: None,
        cached: false,
        no_zkvm: false,
        repeat: 1,
        bench: false,
        cache_dir: std::env::temp_dir(),
    }
}

fn block_hashes(cache: &Cache) -> Vec<H256> {
    cache.blocks.iter().map(|block| block.hash()).collect()
}

#[tokio::test]
async fn erc20_block_contains_only_transfers() -> eyre::Result<()> {
    let opts = options(Workload::Erc20Transfer, 20, 1);
    let cache = build_custom_l1_cache(1, &opts, std::env::temp_dir()).await?;

    assert_eq!(cache.blocks.len(), 1);
    let block = &cache.blocks[0];
    assert_eq!(block.body.transactions.len(), 20);

    // Pure blocks: every transaction is a transfer call to the same token.
    // The deploy and the mints were folded into genesis, never into blocks.
    let TxKind::Call(token) = block.body.transactions[0].to() else {
        panic!("expected a call transaction");
    };
    for tx in &block.body.transactions {
        assert_eq!(tx.to(), TxKind::Call(token));
        assert_eq!(tx.data()[..4], ERC20_TRANSFER_SELECTOR);
    }

    Ok(())
}

#[tokio::test]
async fn batches_carry_the_full_config_in_every_block() -> eyre::Result<()> {
    let opts = options(Workload::EthTransfer, 10, 2);
    let cache = build_custom_l1_cache(3, &opts, std::env::temp_dir()).await?;

    assert_eq!(cache.blocks.len(), 3);
    for block in &cache.blocks {
        assert_eq!(block.body.transactions.len(), 10);
    }

    Ok(())
}

#[tokio::test]
async fn erc20_batches_work_across_blocks() -> eyre::Result<()> {
    let opts = options(Workload::Erc20Transfer, 10, 3);
    let cache = build_custom_l1_cache(2, &opts, std::env::temp_dir()).await?;

    assert_eq!(cache.blocks.len(), 2);
    for block in &cache.blocks {
        assert_eq!(block.body.transactions.len(), 10);
    }

    Ok(())
}

#[tokio::test]
async fn same_seed_produces_identical_blocks() -> eyre::Result<()> {
    let first = build_custom_l1_cache(
        2,
        &options(Workload::EthTransfer, 30, 42),
        std::env::temp_dir(),
    )
    .await?;
    let second = build_custom_l1_cache(
        2,
        &options(Workload::EthTransfer, 30, 42),
        std::env::temp_dir(),
    )
    .await?;
    let other_seed = build_custom_l1_cache(
        2,
        &options(Workload::EthTransfer, 30, 43),
        std::env::temp_dir(),
    )
    .await?;

    assert_eq!(block_hashes(&first), block_hashes(&second));
    assert_ne!(block_hashes(&first), block_hashes(&other_seed));

    Ok(())
}

#[tokio::test]
async fn loop_workloads_hit_their_gas_budget() -> eyre::Result<()> {
    // keccak, mulmod and ecrecover should each burn close to --tx-gas per tx.
    for workload in [Workload::Keccak, Workload::Mulmod, Workload::Ecrecover] {
        let mut opts = options(workload.clone(), 4, 9);
        opts.tx_gas = 2_000_000;
        let cache = build_custom_l1_cache(1, &opts, std::env::temp_dir()).await?;
        let block = &cache.blocks[0];
        let avg = block.header.gas_used / block.body.transactions.len() as u64;
        let lower = 2_000_000 * 95 / 100;
        assert!(
            avg >= lower && avg <= 2_000_000,
            "{} avg gas {avg} not within 5% of 2_000_000",
            workload.name()
        );
    }
    Ok(())
}

#[tokio::test]
async fn sstore_fresh_grows_state_across_blocks() -> eyre::Result<()> {
    let mut opts = options(Workload::SstoreFresh, 3, 11);
    opts.tx_gas = 1_000_000;
    let cache = build_custom_l1_cache(2, &opts, std::env::temp_dir()).await?;
    assert_eq!(cache.blocks.len(), 2);
    // Each block writes fresh slots, so both blocks consume real gas.
    for block in &cache.blocks {
        assert!(block.header.gas_used > 0);
    }
    Ok(())
}

#[tokio::test]
async fn sload_cold_produces_a_larger_witness_than_transfers() -> eyre::Result<()> {
    let mut sload_opts = options(Workload::SloadCold, 4, 13);
    sload_opts.tx_gas = 1_000_000;
    sload_opts.prestate_slots = 5_000;
    let sload = build_custom_l1_cache(1, &sload_opts, std::env::temp_dir()).await?;
    let transfers = build_custom_l1_cache(
        1,
        &options(Workload::EthTransfer, 4, 13),
        std::env::temp_dir(),
    )
    .await?;

    // Cold reads pull storage-trie nodes into the witness; transfers don't.
    assert!(
        sload.witness.state.len() > transfers.witness.state.len(),
        "sload-cold witness ({}) should exceed transfers witness ({})",
        sload.witness.state.len(),
        transfers.witness.state.len()
    );
    Ok(())
}

#[tokio::test]
async fn uniswap_swaps_succeed_within_expected_gas_band() -> eyre::Result<()> {
    let opts = options(Workload::UniswapV2Swap, 10, 19);
    let cache = build_custom_l1_cache(1, &opts, std::env::temp_dir()).await?;

    let block = &cache.blocks[0];
    // Pure block: 10 swaps, no setup transactions.
    assert_eq!(block.body.transactions.len(), 10);
    for tx in &block.body.transactions {
        assert!(matches!(tx.to(), TxKind::Call(_)));
    }

    // A V2 swap (transferFrom + getReserves + swap + transfer) lands in a
    // characteristic gas band. Anything outside it means the vendored bytecode
    // reverted or short-circuited.
    let avg = block.header.gas_used / block.body.transactions.len() as u64;
    assert!(
        (60_000..=200_000).contains(&avg),
        "uniswap swap avg gas {avg} outside the expected band"
    );
    Ok(())
}

#[tokio::test]
async fn contract_deploy_block_contains_only_creates() -> eyre::Result<()> {
    let mut opts = options(Workload::ContractDeploy, 5, 17);
    opts.deploy_code_size = 4_096;
    let cache = build_custom_l1_cache(1, &opts, std::env::temp_dir()).await?;
    let block = &cache.blocks[0];
    assert_eq!(block.body.transactions.len(), 5);
    for tx in &block.body.transactions {
        assert_eq!(tx.to(), TxKind::Create);
    }
    Ok(())
}

#[tokio::test]
async fn cached_rerun_matches_the_original_block() -> eyre::Result<()> {
    let dir = std::env::temp_dir().join("ethrex_replay_cache_test");
    std::fs::create_dir_all(&dir)?;

    let mut opts = options(Workload::Erc20Transfer, 20, 77);
    // First run builds and persists.
    let original = load_or_build_custom_l1_cache(1, &opts, dir.clone()).await?;
    // Second run loads from cache instead of rebuilding.
    opts.cached = true;
    let reloaded = load_or_build_custom_l1_cache(1, &opts, dir.clone()).await?;

    assert_eq!(block_hashes(&original), block_hashes(&reloaded));
    Ok(())
}

#[tokio::test]
async fn gas_target_fills_the_block() -> eyre::Result<()> {
    let mut opts = options(Workload::EthTransfer, 0, 23);
    opts.n_txs = None;
    opts.gas_target = Some(100_000_000);
    let cache = build_custom_l1_cache(1, &opts, std::env::temp_dir()).await?;

    let gas_used = cache.blocks[0].header.gas_used;
    assert!(
        gas_used >= 95_000_000,
        "block gas {gas_used} did not reach 95% of the 100M target"
    );
    Ok(())
}

#[tokio::test]
async fn transactions_are_distributed_across_senders() -> eyre::Result<()> {
    let opts = options(Workload::EthTransfer, 80, 4);
    let cache = build_custom_l1_cache(1, &opts, std::env::temp_dir()).await?;

    let mut senders = HashSet::new();
    for tx in &cache.blocks[0].body.transactions {
        senders.insert(
            tx.sender(&NativeCrypto)
                .map_err(|err| eyre::eyre!("failed to recover sender: {err:?}"))?,
        );
    }
    assert_eq!(senders.len(), 8, "expected round-robin across all senders");

    Ok(())
}
