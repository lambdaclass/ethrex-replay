.PHONY: execute-ci prove-sp1-gpu-ci prove-risc0-gpu-ci execute-sp1-ci execute-risc0-ci

# If RPC URL and Network weren't provided execution will fail.
ifeq ($(origin RPC_URL), undefined)
ifeq ($(origin NETWORK), undefined)
REPLAY_BLOCK_ARGS :=
else
REPLAY_BLOCK_ARGS := $(BLOCK_NUMBER) --cached --network $(NETWORK)
endif
else
REPLAY_BLOCK_ARGS := $(BLOCK_NUMBER) --rpc-url $(RPC_URL)
endif

## Execution block
execute-ci:
	cargo r -r --no-default-features -- block $(REPLAY_BLOCK_ARGS)

prove-sp1-gpu-ci:
	SP1_PROVER=cuda cargo r -r --features "sp1,gpu" -- block --zkvm sp1 --action prove --resource gpu $(REPLAY_BLOCK_ARGS) --bench

prove-risc0-gpu-ci:
	cargo r -r --no-default-features --features "risc0,gpu" -- block --zkvm risc0 --action prove --resource gpu $(REPLAY_BLOCK_ARGS) --bench

execute-sp1-ci:
	cargo r -r --features "sp1" -- block --zkvm sp1 $(REPLAY_BLOCK_ARGS) --bench

execute-risc0-ci:
	cargo r -r --no-default-features --features "risc0" -- block --zkvm risc0 $(REPLAY_BLOCK_ARGS) --bench

update_ethrex:
	cargo update \
	-p ethrex-config \
	-p ethrex-storage \
	-p ethrex-common \
	-p ethrex-vm \
	-p ethrex-levm \
	-p ethrex-rpc \
	-p ethrex-p2p \
	-p ethrex-trie \
	-p ethrex-rlp \
	-p ethrex-blockchain \
	-p ethrex-l2 \
	-p ethrex-storage-rollup \
	-p ethrex-l2-rpc \
	-p ethrex\
	-prover \
	-p guest_program

# --- Profiling targets ---
PROFILE_BLOCK ?= 24443168
PROFILE_REPEAT ?= 10
PROFILE_RPC ?= http://157.180.1.98:8545

.PHONY: profile profile-debug profile-stacks profile-hwcounters profile-samply

## Profile a block with repeat runs
profile:
	cargo run --release -- block $(PROFILE_BLOCK) --no-zkvm --repeat $(PROFILE_REPEAT) --rpc-url $(PROFILE_RPC)

## Profile with debug symbols (for samply/perf)
profile-debug:
	cargo build --profile release-with-debug
	./target/release-with-debug/ethrex-replay block $(PROFILE_BLOCK) --no-zkvm --rpc-url $(PROFILE_RPC)

## Capture folded stacks with perf (Linux only)
profile-stacks:
	cargo build --profile release-with-debug
	perf record -g --call-graph dwarf -F 997 -- ./target/release-with-debug/ethrex-replay block $(PROFILE_BLOCK) --no-zkvm --rpc-url $(PROFILE_RPC)
	perf script | stackcollapse-perf.pl > stacks.folded
	@echo "Folded stacks written to stacks.folded"
	@sort -rn -k2 -t' ' stacks.folded | head -20

## Hardware counters with perf stat (Linux only)
profile-hwcounters:
	cargo build --release
	perf stat -e cycles,instructions,cache-misses,cache-references,branch-misses,L1-dcache-load-misses -- ./target/release/ethrex-replay block $(PROFILE_BLOCK) --no-zkvm --rpc-url $(PROFILE_RPC)

## Profile with samply (macOS, opens browser)
profile-samply:
	cargo build --profile release-with-debug
	samply record ./target/release-with-debug/ethrex-replay block $(PROFILE_BLOCK) --no-zkvm --rpc-url $(PROFILE_RPC)
