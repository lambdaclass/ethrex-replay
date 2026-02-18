# Profiling ethrex Block Execution

This document describes how to profile ethrex's block execution pipeline using `ethrex-replay`'s `--no-zkvm` mode, and how to analyze the results.

## Overview

The `--no-zkvm` flag replays Ethereum blocks through ethrex's `add_block_pipeline` natively, without zkVM overhead. This matches how a real ethrex node processes blocks, making it the right target for profiling ethrex's execution performance.

The `--repeat N` flag runs the block multiple times to get statistically meaningful measurements (median, stddev, percentiles).

## Prerequisites

### Linux Server (recommended)

Linux is required for `perf`-based profiling (folded stacks and hardware counters). macOS can only do repeat runs and samply (GUI-based).

```bash
# Install perf
sudo apt-get install linux-perf

# Allow non-root perf access
sudo sysctl -w kernel.perf_event_paranoid=-1
sudo sysctl -w kernel.kptr_restrict=0

# Install FlameGraph tools (for stackcollapse-perf.pl)
git clone https://github.com/brendangregg/FlameGraph ~/FlameGraph
export PATH=$HOME/FlameGraph:$PATH
```

### Build Profiles

```bash
# Release build (for timing and hardware counters)
cargo build --release

# Release with debug symbols (for perf record / samply)
cargo build --profile release-with-debug
```

The `release-with-debug` profile is defined in `Cargo.toml` — it's a release build with `debuginfo = 2` so `perf` can resolve function names.

## Step 1: Baseline Timing with --repeat

Get statistically stable execution times:

```bash
make profile PROFILE_BLOCK=24443168 PROFILE_REPEAT=10
# or manually:
cargo run --release -- block 24443168 --no-zkvm --repeat 10 --rpc-url http://157.180.1.98:8545
```

This produces:
- Per-run timings
- Min, max, mean, median, stddev
- 95th/99th percentiles (with enough repeats)
- Ggas/s throughput

The first run may be slow due to cache miss (fetching block data from RPC). Subsequent runs use the local cache.

Example output:
```
Individual runs: 59.23ms, 59.62ms, 60.40ms, 59.75ms, 60.12ms, ...
Stats (10 runs): min=59.23ms, max=60.98ms, median=59.75ms, stddev=0.88ms
Throughput: 0.63 Ggas/s
```

## Step 2: Hardware Counters with perf stat

Measure CPU-level metrics:

```bash
make profile-hwcounters PROFILE_BLOCK=24443168
# or manually:
perf stat -e cycles,instructions,cache-misses,cache-references,branch-misses,L1-dcache-load-misses \
  -- ./target/release/ethrex-replay block 24443168 --no-zkvm --rpc-url http://157.180.1.98:8545
```

Key metrics to watch:
- **IPC (instructions per cycle)**: Higher is better. >2.0 is good, <1.0 means the CPU is stalling.
- **Cache miss rate** (`cache-misses / cache-references`): Lower is better. >10% suggests data locality issues.
- **L1-dcache-load-misses**: Absolute count of L1 cache misses. High values indicate poor memory access patterns.
- **Branch misses**: High values suggest unpredictable control flow.

## Step 3: Folded Stacks (CPU Profiling)

Capture where CPU time is spent:

```bash
make profile-stacks PROFILE_BLOCK=24443168
# or manually:
perf record -g --call-graph dwarf -F 997 \
  -- ./target/release-with-debug/ethrex-replay block 24443168 --no-zkvm --rpc-url http://157.180.1.98:8545
perf script | stackcollapse-perf.pl > stacks.folded
```

The `-F 997` flag samples at 997 Hz (a prime to avoid aliasing with periodic program behavior). `--call-graph dwarf` gives accurate stack traces even through optimized code.

### Generating a Flamegraph

```bash
flamegraph.pl stacks.folded > flamegraph.svg
# Open in a browser for interactive exploration
```

### Analyzing Folded Stacks (text-based)

The `stacks.folded` file contains one line per unique stack trace, with a sample count. This is the most useful format for automated analysis.

#### Top hotspots by leaf function (self time)

```bash
# Aggregate samples by the function at the top of the stack (where CPU was actually executing)
perl -ne 'chomp; if (/^(.*)\s+(\d+)$/) {
  @frames = split(/;/, $1);
  $leaf = $frames[-1];
  $total{$leaf} += $2;
} END {
  for (sort { $total{$b} <=> $total{$a} } keys %total) {
    printf "%12d  %s\n", $total{$_}, $_;
  }
}' stacks.folded | head -30
```

#### Who calls a specific function

```bash
# Find which ethrex functions trigger a specific hotspot (e.g., KeccakF1600)
grep 'KeccakF1600' stacks.folded | perl -ne 'chomp; if (/^(.*)\s+(\d+)$/) {
  @f = split(/;/, $1);
  for my $i (0..$#f) {
    if ($f[$i] =~ /KeccakF1600/) {
      for my $j (reverse 0..$i-1) {
        if ($f[$j] =~ /ethrex/) { $total{$f[$j]} += $2; last; }
      }
      last;
    }
  }
} END {
  for (sort { $total{$b} <=> $total{$a} } keys %total) {
    printf "%12d  %s\n", $total{$_}, $_;
  }
}' | head -10
```

#### Module-level breakdown

```bash
# Which ethrex crate is consuming the most time
perl -ne 'chomp; if (/^(.*)\s+(\d+)$/) {
  @f = split(/;/, $1);
  my $found = 0;
  for my $frame (@f) {
    if ($frame =~ /(ethrex_\w+)::/) { $total{$1} += $2; $found = 1; last; }
  }
  $total{"<other>"} += $2 unless $found;
} END {
  my $grand = 0; $grand += $total{$_} for keys %total;
  for (sort { $total{$b} <=> $total{$a} } keys %total) {
    printf "%12d  %5.1f%%  %s\n", $total{$_}, 100*$total{$_}/$grand, $_;
  }
}' stacks.folded
```

## Step 4: macOS Profiling with samply

On macOS, `perf` is not available. Use `samply` instead:

```bash
cargo install samply
make profile-samply PROFILE_BLOCK=24443168
```

This opens a browser-based profiler UI. Useful for interactive exploration but not for automated/text-based analysis.

## Interpreting Results

### What to look for

1. **Leaf function hotspots**: The functions where the CPU is actually spending time (not just calling other functions). These are the direct optimization targets.
2. **Allocation pressure**: `malloc`, `cfree`, `realloc` in the top hotspots indicates excessive heap allocation. Trace callers to find which ethrex code is allocating.
3. **Data structure overhead**: `BTreeMap::find_key_index`, `SliceOrd::compare` indicate expensive key lookups. Consider whether a HashMap would suffice.
4. **I/O vs execution**: Distinguish between cache loading (serde_json), state preparation, and actual block execution. Only the execution portion is relevant for ethrex core optimization.

### Phase separation

The profiling captures the entire program lifecycle. Time is distributed across phases:

| Phase | What it includes |
|-------|-----------------|
| Cache I/O | JSON deserialization of witness data, hex decoding |
| State preparation | Building in-memory trie from witness, dummy node injection |
| Block execution | `add_block_pipeline`: validation, EVM execution, merkleization, storage |
| Cleanup | Cache saving, process shutdown |

When optimizing ethrex core, focus on the block execution phase. The `[METRIC]` log line from `add_block_pipeline` gives the execution-only timing (e.g., `TIME SPENT: 60 ms`), while the total program time includes all phases.

## Reference: Block 24443168 Baseline

Profiled 2025-02-12 on ethrex-office-4 (32-core AMD, 60GB RAM, Linux):

| Metric | Value |
|--------|-------|
| Block | 24443168 (mainnet) |
| Transactions | 442 |
| Gas used | 37.5M (62% of limit) |
| Execution time (median, 10 runs) | 59.75ms |
| Throughput | 0.63 Ggas/s |
| IPC | 2.54 |
| L1 cache miss rate | 12.56% |

### Top hotspots (leaf function self time)

| Function | Samples | % | What |
|----------|---------|---|------|
| `__KeccakF1600` | 255M | 10.0% | SHA3/Keccak hashing |
| `SliceOrd::compare` | 175M | 6.9% | Byte comparisons in BTreeMap |
| `BTreeMap::find_key_index` | 101M | 4.0% | In-memory storage lookups |
| `cfree` | 89M | 3.5% | Memory deallocation |
| `ethrex_rlp::get_item_with_prefix` | 85M | 3.3% | RLP decoding |
| `serde_json IoRead::next` | 79M | 3.1% | JSON parsing (cache I/O) |
| `realloc` | 68M | 2.7% | Memory reallocation |
| `secp256k1 fe_mul_inner` | 67M | 2.6% | ECDSA signature verification |
| `hex::val` | 57M | 2.2% | Hex string parsing (cache I/O) |
| `malloc` | 49M | 1.9% | Memory allocation |

### Execution breakdown (within add_block_pipeline)

| Function | Samples | What |
|----------|---------|------|
| `get_storage_slot` (SLOAD path) | 75M | Storage reads — 48.5% of blockchain time |
| `handle_merkleization` | 37M | Trie hash computation |
| `get_account_state` | 20M | Account lookups |

### LEVM opcode hotspots

| Opcode | Samples |
|--------|---------|
| SLOAD | 74M |
| STATICCALL | 45M |
| CALL | 10M |
| DELEGATECALL | 2M |

## Makefile Targets Reference

| Target | Description | Requires |
|--------|-------------|----------|
| `make profile` | Repeat runs with stats | Any OS |
| `make profile-debug` | Single run with debug symbols | Any OS |
| `make profile-stacks` | Folded stacks via perf | Linux + perf + FlameGraph |
| `make profile-hwcounters` | CPU hardware counters | Linux + perf |
| `make profile-samply` | GUI profiler | macOS + samply |

Override defaults: `make profile PROFILE_BLOCK=12345 PROFILE_REPEAT=20 PROFILE_RPC=http://localhost:8545`
