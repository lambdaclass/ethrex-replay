//! Baseline workload: plain ETH transfers between externally owned accounts.
//! No EVM execution — measures signature recovery and account trie updates.

use ethrex_common::{U256, types::Transaction, types::TxKind};

use super::{GenCtx, WORKLOAD_TX_GAS, build_eip1559};

pub async fn next_tx(ctx: &mut GenCtx) -> eyre::Result<Transaction> {
    let to = ctx.seeded_address();
    let (signer, nonce) = ctx.take_sender();
    build_eip1559(
        nonce,
        U256::one(),
        Vec::new(),
        TxKind::Call(to),
        WORKLOAD_TX_GAS,
        &signer,
        ctx.chain_id,
    )
    .await
}
