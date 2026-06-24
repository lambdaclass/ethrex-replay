//! Synthetic transaction workloads for `ethrex-replay custom`.
//!
//! Each workload generates blocks composed exclusively of one kind of
//! transaction. Contract provisioning never appears in produced blocks:
//! `prepare_genesis` injects state directly, and `setup_txs` are executed at
//! build time on a scratch chain and folded into the genesis allocation
//! (see `fold_setup_into_genesis` in `cli.rs`).

pub mod bytecode;
pub mod erc20;
pub mod eth_transfer;
pub mod loops;
pub mod uniswap;

use bytes::Bytes;
use clap::ValueEnum;
use ethrex_common::{
    Address, H256, U256,
    types::{EIP1559Transaction, Genesis, GenesisAccount, Transaction, TxKind},
    utils::keccak,
};
use ethrex_l2_rpc::signer::{LocalSigner, Signable, Signer};
use ethrex_storage::Store;
use std::collections::BTreeMap;

/// Gas limit for regular workload transactions (same value the previous
/// `TxBuilder` used).
pub const WORKLOAD_TX_GAS: u64 = 250_000;

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq, Default)]
pub enum Workload {
    #[default]
    EthTransfer,
    Erc20Transfer,
    UniswapV2Swap,
    ContractDeploy,
    Keccak,
    Mulmod,
    Ecrecover,
    SstoreFresh,
    SloadCold,
}

impl Workload {
    /// Human-readable name, used in reports and logs.
    pub fn name(&self) -> &'static str {
        match self {
            Workload::EthTransfer => "eth-transfer",
            Workload::Erc20Transfer => "erc20-transfer",
            Workload::UniswapV2Swap => "uniswap-v2-swap",
            Workload::ContractDeploy => "contract-deploy",
            Workload::Keccak => "keccak",
            Workload::Mulmod => "mulmod",
            Workload::Ecrecover => "ecrecover",
            Workload::SstoreFresh => "sstore-fresh",
            Workload::SloadCold => "sload-cold",
        }
    }

    /// Inject state the workload needs directly into the genesis allocation
    /// (synthetic contracts, pre-seeded storage). Third-party contract state
    /// is provisioned through `setup_txs` instead.
    pub fn prepare_genesis(&self, genesis: &mut Genesis, ctx: &GenCtx) {
        match self {
            Workload::EthTransfer
            | Workload::Erc20Transfer
            | Workload::UniswapV2Swap
            | Workload::ContractDeploy => {}
            Workload::Keccak
            | Workload::Mulmod
            | Workload::Ecrecover
            | Workload::SstoreFresh
            | Workload::SloadCold => loops::prepare_genesis(self, genesis, ctx),
        }
    }

    /// Transactions executed at build time on a scratch chain and folded into
    /// genesis. They never appear in produced blocks.
    pub async fn setup_txs(&self, ctx: &mut GenCtx) -> eyre::Result<Vec<Transaction>> {
        match self {
            Workload::Erc20Transfer => erc20::setup_txs(ctx).await,
            Workload::UniswapV2Swap => uniswap::setup_txs(ctx).await,
            _ => Ok(Vec::new()),
        }
    }

    /// Build the next workload transaction. These are the only transactions
    /// that land in produced blocks.
    pub async fn next_tx(&self, ctx: &mut GenCtx) -> eyre::Result<Transaction> {
        match self {
            Workload::EthTransfer => eth_transfer::next_tx(ctx).await,
            Workload::Erc20Transfer => erc20::next_tx(ctx).await,
            Workload::UniswapV2Swap => uniswap::next_tx(ctx).await,
            Workload::ContractDeploy => loops::deploy_next_tx(ctx).await,
            Workload::Keccak | Workload::Mulmod | Workload::Ecrecover => {
                loops::call_next_tx(self, ctx, Vec::new()).await
            }
            Workload::SstoreFresh => loops::stateful_next_tx(self, ctx).await,
            Workload::SloadCold => loops::stateful_next_tx(self, ctx).await,
        }
    }

    /// Approximate gas used per workload transaction, used to size blocks for
    /// `--gas-target`. Loop workloads burn close to their `tx_gas` budget;
    /// the rest are empirical per-transaction costs.
    pub fn estimated_tx_gas(&self, params: &WorkloadParams) -> u64 {
        match self {
            Workload::EthTransfer => 21_000,
            Workload::Erc20Transfer => 52_000,
            Workload::UniswapV2Swap => 85_000,
            Workload::ContractDeploy => 60_000 + 210 * params.deploy_code_size as u64,
            Workload::Keccak
            | Workload::Mulmod
            | Workload::Ecrecover
            | Workload::SstoreFresh
            | Workload::SloadCold => params.tx_gas,
        }
    }

    /// Compatibility seam for L2 custom blocks, which still use a single
    /// signer with externally tracked nonces and random recipients.
    pub async fn build_l2_tx(
        &self,
        nonce: u64,
        signer: &Signer,
        chain_id: u64,
    ) -> eyre::Result<Transaction> {
        match self {
            Workload::EthTransfer => {
                build_eip1559(
                    nonce,
                    U256::one(),
                    Vec::new(),
                    TxKind::Call(Address::random()),
                    WORKLOAD_TX_GAS,
                    signer,
                    chain_id,
                )
                .await
            }
            other => {
                eyre::bail!("{} is not supported for L2 custom blocks yet", other.name())
            }
        }
    }
}

/// Workload parameters that come from CLI flags.
#[derive(Clone, Copy)]
pub struct WorkloadParams {
    /// Per-transaction gas budget for loop workloads.
    pub tx_gas: u64,
    /// Deployed runtime size for `contract-deploy`.
    pub deploy_code_size: usize,
    /// Pre-seeded storage slots for `sload-cold`.
    pub prestate_slots: u64,
}

impl Default for WorkloadParams {
    fn default() -> Self {
        Self {
            tx_gas: 1_000_000,
            deploy_code_size: 1_024,
            prestate_slots: 100_000,
        }
    }
}

/// Shared generation context: seed-derived senders, per-sender nonces and the
/// deterministic value stream. State threads across all blocks of a batch and
/// is never reset per block.
pub struct GenCtx {
    pub chain_id: u64,
    seed: u64,
    senders: Vec<Signer>,
    next_nonces: Vec<u64>,
    next_sender: usize,
    stream_counter: u64,
    /// Storage-slot cursor for `sstore-fresh`/`sload-cold`, threaded across
    /// blocks so each transaction touches disjoint slots.
    slot_cursor: u64,
    pub params: WorkloadParams,
    /// ERC20 token address, set by the erc20 workload during setup.
    pub token_address: Option<Address>,
    /// Uniswap addresses, set by the uniswap workload during setup.
    pub uniswap: Option<uniswap::UniswapDeployment>,
}

impl GenCtx {
    pub fn new(
        chain_id: u64,
        seed: u64,
        n_senders: u64,
        params: WorkloadParams,
    ) -> eyre::Result<Self> {
        eyre::ensure!(n_senders > 0, "--n-senders must be at least 1");
        let senders = (0..n_senders)
            .map(|index| derive_signer(seed, index))
            .collect::<eyre::Result<Vec<_>>>()?;
        Ok(Self {
            chain_id,
            seed,
            next_nonces: vec![0; senders.len()],
            senders,
            next_sender: 0,
            stream_counter: 0,
            slot_cursor: 0,
            params,
            token_address: None,
            uniswap: None,
        })
    }

    /// Reserve `advance` slots starting at the current cursor and return the
    /// reserved base. Reserving (not just reading) guarantees the next
    /// transaction's slots are disjoint from this one's.
    pub fn reserve_slots(&mut self, advance: u64) -> u64 {
        let base = self.slot_cursor;
        self.slot_cursor += advance.max(1);
        base
    }

    pub fn senders(&self) -> &[Signer] {
        &self.senders
    }

    /// Fund every sender in the genesis allocation. EIP-1559 validity requires
    /// balance >= gas_limit * max_fee_per_gas per transaction and workload
    /// transactions use max_fee_per_gas = u64::MAX, so senders get the same
    /// huge balance the devnet rich accounts have.
    pub fn fund_senders(&self, genesis: &mut Genesis) {
        for sender in &self.senders {
            genesis
                .alloc
                .entry(sender.address())
                .or_insert_with(|| GenesisAccount {
                    code: Bytes::new(),
                    storage: BTreeMap::new(),
                    balance: U256::from(10u128.pow(30)),
                    nonce: 0,
                });
        }
    }

    /// Read each sender's starting nonce from the store. Senders that signed
    /// folded setup transactions start above zero, so nonces must come from
    /// the enriched genesis state, not from a hardcoded zero.
    pub async fn init_nonces(&mut self, store: &Store) -> eyre::Result<()> {
        let latest = store.get_latest_block_number().await?;
        for (i, sender) in self.senders.iter().enumerate() {
            self.next_nonces[i] = store
                .get_nonce_by_account_address(latest, sender.address())
                .await?
                .unwrap_or(0);
        }
        Ok(())
    }

    /// Next sender (round-robin) together with its next nonce.
    pub fn take_sender(&mut self) -> (Signer, u64) {
        let i = self.next_sender;
        self.next_sender = (self.next_sender + 1) % self.senders.len();
        let nonce = self.next_nonces[i];
        self.next_nonces[i] += 1;
        (self.senders[i].clone(), nonce)
    }

    /// Deterministic address from the seeded stream (replaces
    /// `Address::random()` so identical invocations produce identical blocks).
    pub fn seeded_address(&mut self) -> Address {
        let hash = self.seeded_hash(b"address");
        Address::from_slice(&hash.as_bytes()[12..])
    }

    fn seeded_hash(&mut self, domain: &[u8]) -> H256 {
        let mut data = Vec::with_capacity(8 + domain.len() + 8);
        data.extend_from_slice(&self.seed.to_be_bytes());
        data.extend_from_slice(domain);
        data.extend_from_slice(&self.stream_counter.to_be_bytes());
        self.stream_counter += 1;
        keccak(data)
    }
}

/// Derive a sender key as keccak(seed ‖ domain ‖ index). The attempt counter
/// handles the negligible chance that a hash is not a valid secp256k1 scalar.
fn derive_signer(seed: u64, index: u64) -> eyre::Result<Signer> {
    for attempt in 0u64..16 {
        let mut data = Vec::new();
        data.extend_from_slice(&seed.to_be_bytes());
        data.extend_from_slice(b"ethrex-replay-sender");
        data.extend_from_slice(&index.to_be_bytes());
        data.extend_from_slice(&attempt.to_be_bytes());
        let hash = keccak(data);
        if let Ok(private_key) = hex::encode(hash.as_bytes()).parse() {
            return Ok(Signer::Local(LocalSigner::new(private_key)));
        }
    }
    eyre::bail!("could not derive a valid sender key for seed {seed}, index {index}")
}

pub(crate) async fn build_eip1559(
    nonce: u64,
    value: U256,
    calldata: Vec<u8>,
    to: TxKind,
    gas_limit: u64,
    signer: &Signer,
    chain_id: u64,
) -> eyre::Result<Transaction> {
    Transaction::EIP1559Transaction(EIP1559Transaction {
        nonce,
        value,
        gas_limit,
        max_fee_per_gas: u64::MAX,
        max_priority_fee_per_gas: 10,
        chain_id,
        data: calldata.into(),
        to,
        ..Default::default()
    })
    .sign(signer)
    .await
    .map_err(|err| eyre::eyre!("failed to sign transaction: {err}"))
}
