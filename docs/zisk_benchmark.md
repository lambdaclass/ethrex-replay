# ZisK Execution Benchmark — Custom L1 Workloads

ZisK-zkVM execution cost of every `ethrex-replay custom` L1 workload, measured
across 1/10/100/1000 transactions per block. The headline metric is **ZisK
steps per gas** — how much zkVM work each kind of transaction costs, normalized
by EVM gas. It exposes where gas pricing and zkVM proving cost diverge.

## TL;DR

At equal gas, workloads differ by **~130×** in ZisK cost:

| Rank | Workload | ZisK steps/gas | What it stresses |
|------|----------|---------------:|------------------|
| 1 (cheapest) | `contract-deploy` | **0.89** | CREATE + code deposit |
| 2 | `sstore-fresh` | **1.16** | Cold SSTORE (state growth) |
| 3 | `erc20-transfer` | **2.60** | Warm storage + logs |
| 4 | `eth-transfer` | **3.66** | Signature recovery + trie |
| 5 | `ecrecover` | **5.59** | secp256k1 recover precompile |
| 6 | `uniswap-v2-swap` | **5.76** | 4-contract DeFi call graph |
| 7 | `sload-cold` | **6.06** | Cold SLOAD (witness growth) |
| 8 | `keccak` | **11.36** | KECCAK256 hashing |
| 9 (priciest) | `mulmod` | **116.92** | 256-bit modular arithmetic |

(steps/gas at the largest measured block; see per-workload tables for scaling.)

**Two findings stand out:**
- **Gas mis-prices zkVM cost in both directions.** State-growth ops are *gas-expensive but step-cheap* (`sstore-fresh` 1.16, `contract-deploy` 0.89), while 256-bit arithmetic is *gas-cheap but step-expensive* (`mulmod` 117). A gas-metered block of `mulmod` is ~130× harder to prove than a gas-equal block of deploys.
- **Steps/gas falls as blocks grow**, because fixed per-block/per-tx overhead (the witness, signature recovery, block setup) amortizes. A 1-tx block looks 3–7× more expensive per gas than a full one (e.g. `eth-transfer` 25.4 → 3.66).

## Methodology

- **Backend:** ZisK 0.16.1, **execution** (the `ziskemu` emulator), CPU-only. No proving (the host has no GPU). ZisK *steps* are the proxy for proving cost.
- **Host:** ethrex-office-5 — 32 cores, 60 GB RAM, Debian 13, no GPU.
- **ethrex:** rev `55e696e` (ziskos 0.16.1). Built `--features "zisk-build-elf,ci" --locked` (`zisk-build-elf` compiles the real guest; `ci` skips the proving-only `rom-setup`).
- **Per case:** `ethrex-replay custom block --tx <workload> --n-txs <n> [--tx-gas G] --zkvm zisk --action execute` generates the block and serializes the guest input; then `ziskemu --elf <guest> --inputs <input> -m` reports `steps`, emulation `duration`, and throughput (`Msteps/s`). Genesis gas limit is raised so each block reaches its true size (no truncation).
- **Sizing:**
  - Transaction-shaped workloads (`eth-transfer`, `erc20-transfer`, `uniswap-v2-swap`, `contract-deploy`) scale naturally by transaction count.
  - Compute/witness loop workloads (`keccak`, `mulmod`, `ecrecover`, `sload-cold`) use `--tx-gas 60000`, so 1000 txs ≈ a **mainnet-sized 60M-gas block**.
  - `sstore-fresh` uses `--tx-gas 200000`: a single cold SSTORE is 22.1k gas, so a 60k budget can't fit one after intrinsic + the loop's OOG-safety margin. 200k lets each tx perform several real cold writes. Its absolute gas is therefore larger, but steps/gas (the comparison metric) is budget-independent.
- All blocks are deterministic (`--seed 0`), single-block, the LocalDevnet (Prague) chain.

## Per-workload scaling

Columns: transactions, total block gas, ZisK steps, steps/gas, emulation time, throughput.

### eth-transfer — baseline (sig recovery + account trie)
| txs | gas | steps | steps/gas | emu (s) | Msteps/s |
|----:|----:|------:|----------:|--------:|---------:|
| 1 | 21,000 | 533,567 | 25.41 | 0.014 | 37.7 |
| 10 | 210,000 | 1,419,230 | 6.76 | 0.104 | 13.7 |
| 100 | 2,100,000 | 8,181,649 | 3.90 | 0.977 | 8.4 |
| 1000 | 21,000,000 | 76,796,961 | 3.66 | 9.71 | 7.9 |

### erc20-transfer — warm storage + logs
| txs | gas | steps | steps/gas | emu (s) | Msteps/s |
|----:|----:|------:|----------:|--------:|---------:|
| 1 | 52,117 | 689,784 | 13.24 | 0.015 | 44.9 |
| 10 | 521,158 | 2,147,724 | 4.12 | 0.110 | 19.6 |
| 100 | 5,211,616 | 14,031,894 | 2.69 | 1.030 | 13.6 |
| 1000 | 52,116,076 | 135,727,533 | 2.60 | 10.17 | 13.3 |

### uniswap-v2-swap — DeFi composite (router → pair → token)
| txs | gas | steps | steps/gas | emu (s) | Msteps/s |
|----:|----:|------:|----------:|--------:|---------:|
| 1 | 77,249 | 1,202,832 | 15.57 | 0.019 | 62.7 |
| 10 | 892,190 | 5,581,771 | 6.26 | 0.136 | 41.0 |
| 100 | 7,844,600 | 45,446,116 | 5.79 | 1.277 | 35.6 |
| 1000 | 77,368,700 | 446,021,675 | 5.76 | 12.55 | 35.5 |

### contract-deploy — CREATE + code deposit (1 KiB runtime)
| txs | gas | steps | steps/gas | emu (s) | Msteps/s |
|----:|----:|------:|----------:|--------:|---------:|
| 1 | 262,354 | 703,661 | 2.68 | 0.015 | 47.8 |
| 10 | 2,623,540 | 2,967,431 | 1.13 | 0.113 | 26.4 |
| 100 | 26,235,400 | 23,821,844 | 0.91 | 1.064 | 22.4 |
| 1000 | 262,354,000 | 233,192,076 | 0.89 | 10.55 | 22.1 |

### keccak — KECCAK256 hashing (`--tx-gas 60000`)
| txs | gas | steps | steps/gas | emu (s) | Msteps/s |
|----:|----:|------:|----------:|--------:|---------:|
| 1 | 51,965 | 1,061,598 | 20.43 | 0.018 | 60.7 |
| 10 | 519,650 | 6,521,403 | 12.55 | 0.137 | 47.6 |
| 100 | 5,196,500 | 59,404,993 | 11.43 | 1.305 | 45.5 |
| 1000 | 51,965,000 | 590,169,032 | 11.36 | 13.06 | 45.2 |

### mulmod — 256-bit modular multiplication (`--tx-gas 60000`)
| txs | gas | steps | steps/gas | emu (s) | Msteps/s |
|----:|----:|------:|----------:|--------:|---------:|
| 1 | 50,606 | 6,407,708 | 126.62 | 0.061 | 105.8 |
| 10 | 506,060 | 59,806,392 | 118.18 | 0.572 | 104.5 |
| 100 | 5,060,600 | 592,078,640 | 117.00 | 5.682 | 104.2 |
| 1000 | 50,606,000 | 5,916,728,986 | 116.92 | 56.32 | 105.1 |

### ecrecover — secp256k1 recover precompile (`--tx-gas 60000`)
| txs | gas | steps | steps/gas | emu (s) | Msteps/s |
|----:|----:|------:|----------:|--------:|---------:|
| 1 | 52,637 | 781,133 | 14.84 | 0.105 | 7.4 |
| 10 | 526,370 | 3,578,335 | 6.80 | 1.014 | 3.5 |
| 100 | 5,263,700 | 29,844,393 | 5.67 | 10.05 | 3.0 |
| 1000 | 52,637,000 | 294,433,692 | 5.59 | 100.14 | 2.9 |

> `ecrecover` has a *low* step count per gas but the **lowest emulation
> throughput** (2.9 Msteps/s vs 100+ for `mulmod`): each step is heavy, so its
> 1000-tx block took 100 s to emulate despite only ~294M steps. This is the
> workload to watch for proving wall-time.

### sstore-fresh — cold SSTORE / state growth (`--tx-gas 200000`)
| txs | gas | steps | steps/gas | emu (s) | Msteps/s |
|----:|----:|------:|----------:|--------:|---------:|
| 1 | 154,357 | 649,378 | 4.21 | 0.015 | 42.9 |
| 10 | 1,543,678 | 2,418,376 | 1.57 | 0.112 | 21.6 |
| 100 | 15,437,800 | 18,338,201 | 1.19 | 1.064 | 17.2 |
| 1000 | 154,380,664 | 178,979,716 | 1.16 | 10.57 | 16.9 |

### sload-cold — cold SLOAD / witness growth (`--tx-gas 60000`)
| txs | gas | steps | steps/gas | emu (s) | Msteps/s |
|----:|----:|------:|----------:|--------:|---------:|
| 1 | 51,218 | 1,113,041 | 21.73 | 0.018 | 62.6 |
| 10 | 512,300 | 5,854,546 | 11.43 | 0.131 | 44.6 |
| 100 | 5,124,080 | 41,743,937 | 8.15 | 1.200 | 34.8 |
| 1000 | 51,241,844 | 310,760,414 | 6.06 | 11.29 | 27.5 |

## Interpreting the numbers

- **For proving-cost estimation**, `steps/gas` is the number to use, not gas. A
  gas-full mainnet block dominated by arithmetic/hashing precompiles is far
  costlier to prove than its gas suggests; a block of transfers, ERC20 activity
  or state writes is cheaper.
- **`mulmod` is the outlier** (117 steps/gas): 256-bit modular multiplication
  has no ZisK precompile acceleration here, so it dominates zkVM cost while
  being cheap in gas. Worth flagging for any 256-bit-math-heavy contract.
- **`ecrecover` trades step count for step weight.** Few steps per gas, but the
  slowest to emulate per step. Step count alone understates its cost.
- **State growth is cheap in steps.** Both `contract-deploy` and `sstore-fresh`
  sit below 1.2 steps/gas — the EVM prices state expansion highly in gas
  (22.1k/slot, 200/byte) for reasons unrelated to zkVM proving work.
- **Block fill matters.** Per-gas cost roughly halves from a 1-tx to a full
  block as fixed witness/recovery/setup overhead amortizes; benchmark against
  realistically full blocks.

## Caveats

- **Execution, not proving.** Steps are a proxy for proving cost; actual proof
  generation (memory, FFT, recursion) may scale differently per workload and
  needs a GPU host to measure. These numbers rank *relative* zkVM cost.
- **`sstore-fresh` uses a larger per-tx budget** (200k vs 60k) because one cold
  SSTORE exceeds a 60k working budget; its absolute gas/steps are larger but
  steps/gas is comparable.
- **`contract-deploy` 1000-tx is a 262M-gas block** (4× mainnet) — included for
  scaling, not realism. Single-block, LocalDevnet (Prague), deterministic seed.
- Emulation times are wall-clock on a 32-core CPU box and are
  machine-dependent; `steps` and `steps/gas` are not.
