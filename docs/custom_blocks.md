# Custom L1 Blocks

`ethrex-replay custom` generates synthetic L1 blocks in-process and executes or
proves them, with no live network or RPC endpoint. It is a benchmark-grade
workload generator: each block is filled with a single chosen workload so you
can measure how ethrex (and the zkVM backends) handle a specific kind of load —
plain transfers, ERC20 activity, a Uniswap swap mix, contract deployments, or
targeted compute/state stress.

## Quick start

```bash
# A block of 1,000 ETH transfers, executed without a zkVM.
ethrex-replay custom block --tx eth-transfer --n-txs 1000 --no-zkvm

# A block of ERC20 transfers, executed (default action) on the Exec backend.
ethrex-replay custom block --tx erc20-transfer --n-txs 500

# A 3-block batch of Uniswap V2 swaps.
ethrex-replay custom batch --n-blocks 3 --tx uniswap-v2-swap --n-txs 200

# Fill a block to ~100M gas of keccak hashing and profile it over 10 runs.
ethrex-replay custom block --tx keccak --gas-target 100000000 --no-zkvm --repeat 10
```

Example output (the per-block line, followed by the batch summary):

```
[local-devnet] Block: 1, Gas: 9934290, #Txs: 10, Execution Time: 00s 174ms
Custom workload 'keccak' (seed 0): 1 block(s), 10 transaction(s), 9934290 total gas
```

## Workload catalog

| Workload | What it stresses | Per-tx gas | Knobs |
|----------|------------------|-----------|-------|
| `eth-transfer` | Signature recovery and account-trie updates; no EVM | ~21k | — |
| `erc20-transfer` | Light EVM, warm storage reads/writes, logs | ~52k | — |
| `uniswap-v2-swap` | Realistic DeFi: 4-contract call graph (router → token.transferFrom / pair.swap → token.transfer), fixed-point math | ~85k | — |
| `contract-deploy` | CREATE, initcode execution, code-deposit cost, code-growth witness | tunable | `--deploy-code-size` |
| `keccak` | KECCAK256 hashing throughput | `--tx-gas` | `--tx-gas` |
| `mulmod` | 256-bit modular multiplication (worst-case operands) | `--tx-gas` | `--tx-gas` |
| `ecrecover` | secp256k1 recovery precompile (valid signature, real work) | `--tx-gas` | `--tx-gas` |
| `sstore-fresh` | State growth: every write hits a fresh keccak-spread slot | `--tx-gas` | `--tx-gas` |
| `sload-cold` | Witness size: cold reads over pre-seeded storage | `--tx-gas` | `--tx-gas`, `--prestate-slots` |

The compute and state workloads (`keccak`, `mulmod`, `ecrecover`, `sstore-fresh`,
`sload-cold`) all run the same bytecode loop template, sized so each transaction
burns close to `--tx-gas`. Identical gas, very different cost profiles — for
example, at 1M gas/tx a `mulmod` block takes far longer to execute than an
`sstore-fresh` block, which is exactly the signal the generator exists to
expose.

## Block sizing

Two mutually exclusive ways to size a block:

- `--n-txs N`: include exactly `N` transactions per block. If they don't all fit
  under the block gas limit, a warning names how many were left out.
- `--gas-target G`: fill each block to roughly `G` gas. The transaction count is
  derived from the workload's per-transaction cost, and the genesis gas limit is
  raised so the target is reachable (a block's gas limit can otherwise only grow
  by 1/1024 per block).

In a batch (`custom batch --n-blocks N`), the sizing applies to **each** block:
`--n-txs 100 --n-blocks 3` produces three 100-transaction blocks, and a single
execution witness spans all three. Generator state (sender nonces, storage-slot
cursors) threads continuously across blocks, so `sstore-fresh`/`sload-cold`
touch disjoint slots in every block and state grows monotonically across the
batch.

## Determinism and reproducibility

- `--seed S` drives all randomness (sender keys, recipients, generated values).
  Two identical invocations produce byte-identical blocks.
- `--n-senders K` (default 8) derives `K` sender accounts from the seed and funds
  them in genesis. Transactions are distributed round-robin so each sender's
  nonces stay sequential.
- Every run persists its cache. Re-run the exact same block without regenerating
  it with `--cached`:

  ```bash
  ethrex-replay custom block --tx erc20-transfer --n-txs 500 --seed 7
  ethrex-replay custom block --tx erc20-transfer --n-txs 500 --seed 7 --cached
  ```

## Replay options

The custom subcommands accept the standard replay options:

- `--no-zkvm` executes through `Blockchain::add_block_pipeline` instead of a zkVM
  backend, and `--repeat N` runs it `N` times and reports timing statistics
  (median, p95, …) — the path used for native performance profiling.
- `--zkvm <backend> --action prove` proves the generated block.
- `--save-program-input <path>` writes the serialized guest `ProgramInput`.
- `--bench` writes the `bench_latest.json` used by the benchmarks pipeline.

`--no-zkvm` currently supports single-block runs only; use `custom block` (not a
multi-block batch) when profiling.

## Architecture

```text
 base LocalDevnet genesis
        │
        ▼
 ┌─────────────────────────────────────────────────────────┐
 │ genesis enrichment (build time, never in a block)       │
 │  • fund seed-derived senders                            │
 │  • prepare_genesis: inject synthetic contracts /        │
 │    pre-seeded storage (compute & state workloads)       │
 │  • setup_txs executed on a scratch chain, the resulting │
 │    account updates folded into genesis.alloc            │
 │    (ERC20 deploy + mints, Uniswap deploy + liquidity)   │
 └─────────────────────────────────────────────────────────┘
        │  enriched genesis = the world
        ▼
 produce N blocks ── each contains ONLY workload transactions
        │
        ▼
 generate execution witness (spans all blocks)
        │
        ▼
 Cache { blocks, witness } ──► execute / prove / profile ──► Report
```

The **pure-blocks invariant** is the central design choice: every block produced
by `custom` contains only transactions of the requested workload. Provisioning
(token deploys, mints, liquidity) never appears as a transaction in a block.
Instead, setup transactions run at build time on a throwaway chain, and their
resulting state (balances, nonces, code, storage) is folded into the genesis
allocation that the real chain starts from. This keeps measured blocks clean and
makes `custom block` produce exactly one block.

For contracts whose storage layout we own (the compute/state loop contracts and
the `sload-cold` pre-state), genesis state is injected directly. For third-party
contracts (ERC20, Uniswap), the genesis fold runs their real setup transactions
so the storage layout is computed by the EVM rather than hand-crafted — correct
by construction.

## Adding a workload

1. Add a variant to the `Workload` enum in `src/workloads/mod.rs` and give it a
   name in `name()`.
2. Implement its behavior. For a contract-call workload, add a module with
   `setup_txs` (folded into genesis) and `next_tx` (the per-block transaction).
   For a compute/state workload, add a bytecode builder in
   `src/workloads/bytecode.rs` and wire it through `src/workloads/loops.rs`.
3. If the workload needs pre-existing state, inject it in `prepare_genesis`
   (direct injection for contracts you author) or provision it in `setup_txs`
   (genesis fold for third-party contracts).
4. Add an estimated per-transaction gas to `estimated_tx_gas` so `--gas-target`
   can size blocks.
5. Add a test in `tests/custom_workloads.rs` asserting the block is pure and the
   transactions execute successfully.

## Prior art

The workload taxonomy and generation techniques draw on existing tools:

- **ethrex `tooling/load_test`** — the `TestToken` ERC20 fixture and `freeMint()`
  setup pattern, and the embedded-bytecode-constant convention.
- **spamoor** (ethpandaops) — the gas-bounded bytecode loop for compute stress,
  the fresh-slot SSTORE pattern, and seed-derived sender keys.
- **EEST `tests/benchmark/`** (ethereum/execution-specs) — the `JumpLoop`
  contract shape, analytic gas-budget sizing, and the cold/warm state-access
  taxonomy.
- **Nethermind `gas-benchmarks`** — the measurement methodology (unmeasured
  setup, repeated runs, gas-per-second reporting).

## Possible follow-ups

Not yet implemented:

- Blob (type-3) transactions and EIP-7702 set-code transactions.
- Weighted mixes of multiple workloads in a single block.
- Consuming EEST benchmark fixtures directly (`custom from-fixture`).
- L2 custom workloads beyond `eth-transfer`.
