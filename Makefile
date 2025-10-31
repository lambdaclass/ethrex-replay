.PHONY: execute-ci prove-sp1-gpu-ci prove-risc0-gpu-ci execute-sp1-ci execute-risc0-ci build update-ethrex-deps

help: ## 📚 Show help for each of the Makefile recipes
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

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
execute-ci: ## Execute for CI
	cargo r -r --no-default-features -- block $(REPLAY_BLOCK_ARGS)

prove-sp1-gpu-ci: ## Prove with SP1 GPU for CI
	SP1_PROVER=cuda cargo r -r --features "sp1,gpu" -- block --zkvm sp1 --action prove --resource gpu $(REPLAY_BLOCK_ARGS) --bench

prove-risc0-gpu-ci: ## Prove with RISC0 GPU for CI
	cargo r -r --no-default-features --features "risc0,gpu" -- block --zkvm risc0 --action prove --resource gpu $(REPLAY_BLOCK_ARGS) --bench

execute-sp1-ci: ## Execute with SP1 for CI
	cargo r -r --features "sp1" -- block --zkvm sp1 $(REPLAY_BLOCK_ARGS) --bench

execute-risc0-ci: ## Execute with RISC0 for CI
	cargo r -r --no-default-features --features "risc0" -- block --zkvm risc0 $(REPLAY_BLOCK_ARGS) --bench

update-ethrex-deps: ## Update ethrex dependencies (auto-detected from Cargo.toml)
	@echo "Scanning Cargo.toml for lambdaclass/ethrex dependencies..."
	@pkgs=$$(grep 'lambdaclass/ethrex' Cargo.toml | sed -E 's/^([^= ]+).*/\1/'); \
	if [ -z "$$pkgs" ]; then \
		echo "No ethrex git dependencies found."; \
		exit 0; \
	fi; \
	flags=""; \
	for p in $$pkgs; do flags="$$flags -p $$p"; done; \
	echo "Running: cargo update$$flags"; \
	cargo update $$flags

build: update-ethrex-deps ## Build project after updating ethrex dependencies
	cargo build --release $(if $(FEATURES),--features "$(FEATURES)",)
