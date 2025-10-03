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
