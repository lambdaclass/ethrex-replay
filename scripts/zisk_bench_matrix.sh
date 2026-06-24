#!/usr/bin/env bash
# ZisK execution benchmark matrix for ethrex-replay custom workloads.
# For each (workload, n_txs) case: generate the block (capturing gas) via the
# zisk execute path, then run `ziskemu -m` for the authoritative step count and
# emulation time. Results are appended to a CSV.
#
# Run on ethrex-office-5 with the ZisK 0.16.1 toolchain on PATH.
set -u

REPLAY_DIR="$HOME/ethrex-replay"
BIN="$REPLAY_DIR/target/release/ethrex-replay"
PROVER_DIR="$HOME/.cargo/git/checkouts/ethrex-b17359103979e28f/55e696e/crates/prover"
ELF="$PROVER_DIR/zkvm-zisk-program"
INPUT="$PROVER_DIR/zisk_input.bin"
OUT="$HOME/zisk_bench_results.csv"
TIMEOUT="${CASE_TIMEOUT:-900}"

# Loop workloads are sized by gas: --tx-gas 60000 so 1000 txs ~= 60M (mainnet).
LOOP_WORKLOADS="keccak mulmod ecrecover sstore-fresh sload-cold"
ALL_WORKLOADS="eth-transfer erc20-transfer uniswap-v2-swap contract-deploy keccak mulmod ecrecover sstore-fresh sload-cold"
COUNTS="1 10 100 1000"

echo "workload,n_txs,tx_gas,total_gas,steps,emu_seconds,msteps_s,steps_per_gas,status" > "$OUT"

is_loop() { case " $LOOP_WORKLOADS " in *" $1 "*) return 0;; *) return 1;; esac; }

for w in $ALL_WORKLOADS; do
  extra=""
  txgas="n/a"
  if is_loop "$w"; then extra="--tx-gas 60000"; txgas="60000"; fi

  for n in $COUNTS; do
    echo ">>> $w n_txs=$n $extra"
    rm -f "$INPUT"

    # 1) Generate block + zisk_input.bin, capture gas. This also runs ziskemu
    #    once internally; we re-run with -m below for the authoritative numbers.
    gen_log=$(cd "$REPLAY_DIR" && timeout "$TIMEOUT" "$BIN" custom block --tx "$w" --n-txs "$n" $extra --zkvm zisk --action execute 2>&1)
    gen_rc=$?
    total_gas=$(echo "$gen_log" | grep -oE "Gas: [0-9]+" | head -1 | grep -oE "[0-9]+")
    total_gas=${total_gas:-0}

    if [ $gen_rc -ne 0 ] || [ ! -f "$INPUT" ]; then
      echo "$w,$n,$txgas,$total_gas,,,,,GEN_FAIL_rc${gen_rc}" >> "$OUT"
      echo "    GEN_FAIL rc=$gen_rc"
      continue
    fi

    # 2) Authoritative emulation: steps + duration + throughput.
    emu=$(timeout "$TIMEOUT" ziskemu --elf "$ELF" --inputs "$INPUT" -m 2>&1)
    emu_rc=$?
    line=$(echo "$emu" | grep -oE "steps=[0-9]+ duration=[0-9.]+ tp=[0-9.]+")
    steps=$(echo "$line" | grep -oE "steps=[0-9]+" | grep -oE "[0-9]+")
    dur=$(echo "$line" | grep -oE "duration=[0-9.]+" | grep -oE "[0-9.]+")
    tp=$(echo "$line" | grep -oE "tp=[0-9.]+" | grep -oE "[0-9.]+")

    if [ $emu_rc -ne 0 ] || [ -z "$steps" ]; then
      echo "$w,$n,$txgas,$total_gas,,,,,EMU_FAIL_rc${emu_rc}" >> "$OUT"
      echo "    EMU_FAIL rc=$emu_rc"
      continue
    fi

    spg=$(awk -v s="$steps" -v g="$total_gas" 'BEGIN{ if (g>0) printf "%.2f", s/g; else print "0" }')
    echo "$w,$n,$txgas,$total_gas,$steps,$dur,$tp,$spg,ok" >> "$OUT"
    echo "    gas=$total_gas steps=$steps emu=${dur}s tp=${tp}Msteps/s steps/gas=$spg"
  done
done

echo "=== DONE; results in $OUT ==="
cat "$OUT"
