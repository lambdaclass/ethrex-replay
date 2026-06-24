//! Loop-template workloads (compute and state stress) plus contract
//! deployment. Compute/state contracts are injected into genesis directly;
//! the deploy workload sends initcode as the workload itself.

use bytes::Bytes;
use ethrex_common::{
    Address, U256,
    types::{Genesis, GenesisAccount, Transaction, TxKind},
};
use std::collections::BTreeMap;

use super::{GenCtx, Workload, build_eip1559, bytecode};

/// Address where compute/state loop contracts are injected in genesis. Chosen
/// high enough to avoid colliding with the devnet's pre-deployed accounts.
pub fn loop_contract_address() -> Address {
    Address::from_low_u64_be(0xC0DE_0001)
}

/// Inject the loop contract (and, for `sload-cold`, its pre-seeded storage).
pub fn prepare_genesis(workload: &Workload, genesis: &mut Genesis, ctx: &GenCtx) {
    let runtime = match workload {
        Workload::Keccak => bytecode::keccak_runtime(),
        Workload::Mulmod => bytecode::mulmod_runtime(),
        Workload::Ecrecover => bytecode::ecrecover_runtime(),
        Workload::SstoreFresh => bytecode::sstore_fresh_runtime(),
        Workload::SloadCold => bytecode::sload_cold_runtime(),
        other => unreachable!("{} is not a loop workload", other.name()),
    };

    let mut storage = BTreeMap::new();
    if matches!(workload, Workload::SloadCold) {
        // Pre-seed sequential cold slots [0, prestate_slots) with a non-zero
        // value so reads traverse a populated storage trie.
        for slot in 0..ctx.params.prestate_slots {
            storage.insert(U256::from(slot), U256::one());
        }
    }

    genesis.alloc.insert(
        loop_contract_address(),
        GenesisAccount {
            code: Bytes::from(runtime),
            storage,
            balance: U256::zero(),
            nonce: 1,
        },
    );
}

/// `contract-deploy`: each transaction creates a contract of the configured
/// runtime size.
pub async fn deploy_next_tx(ctx: &mut GenCtx) -> eyre::Result<Transaction> {
    let code_size = ctx.params.deploy_code_size;
    let initcode = bytecode::deploy_initcode(code_size);
    // CREATE charges 200 gas per deployed byte plus initcode execution.
    let gas_limit = 200_000 + 250 * code_size as u64;
    let chain_id = ctx.chain_id;
    let (signer, nonce) = ctx.take_sender();
    build_eip1559(
        nonce,
        U256::zero(),
        initcode,
        TxKind::Create,
        gas_limit,
        &signer,
        chain_id,
    )
    .await
}

/// `keccak`/`mulmod`/`ecrecover`: call the loop contract with the per-tx gas
/// budget and the given calldata.
pub async fn call_next_tx(
    _workload: &Workload,
    ctx: &mut GenCtx,
    calldata: Vec<u8>,
) -> eyre::Result<Transaction> {
    let gas_limit = ctx.params.tx_gas;
    let chain_id = ctx.chain_id;
    let (signer, nonce) = ctx.take_sender();
    build_eip1559(
        nonce,
        U256::zero(),
        calldata,
        TxKind::Call(loop_contract_address()),
        gas_limit,
        &signer,
        chain_id,
    )
    .await
}

/// `sstore-fresh`/`sload-cold`: call the loop contract passing a fresh slot
/// base in calldata, reserving enough slots that the next transaction's range
/// is disjoint.
pub async fn stateful_next_tx(workload: &Workload, ctx: &mut GenCtx) -> eyre::Result<Transaction> {
    // Upper bound on slots a transaction can touch: budget / minimum per-slot
    // cost. Reserving this many guarantees disjoint ranges (gaps are fine).
    let per_slot_cost = match workload {
        Workload::SstoreFresh => 20_000,
        Workload::SloadCold => 2_000,
        other => unreachable!("{} is not a stateful loop workload", other.name()),
    };
    let advance = ctx.params.tx_gas / per_slot_cost + 1;
    let base = ctx.reserve_slots(advance);
    let calldata = U256::from(base).to_big_endian().to_vec();
    call_next_tx(workload, ctx, calldata).await
}
