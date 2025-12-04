#!/usr/bin/env bash

# Map gas used per block
declare -A GAS_USED=(
  [23919400]=41075722
  [23919500]=40237085
  [23919600]=24064259
  [23919700]=20862238
  [23919800]=31813109
  [23919900]=22917739
  [23920000]=37256487
  [23920100]=33542307
  [23920200]=22994047
  [23920300]=53950967
)

BLOCKS=(
  23919400 23919500 23919600 23919700 23919800
  23919900 23920000 23920100 23920200 23920300
)

echo "| Block | Gas Used | Steps | Duration (s) | TP (Msteps/s) | Freq (MHz) | Clocks/step |"
echo "|-------|-----------|--------|--------------|----------------|------------|--------------|"

for BLOCK in "${BLOCKS[@]}"; do
    INPUT="generated_inputs/ethrex_mainnet_${BLOCK}_input.bin"

    OUT=$(ziskemu -e riscv64ima-zisk-elf -i "$INPUT" -m 2>&1)

    LINE=$(echo "$OUT" | grep "process_rom()")

    STEPS=$(echo "$LINE" | sed -n 's/.*steps=\([0-9]*\).*/\1/p')
    DURATION=$(echo "$LINE" | sed -n 's/.*duration=\([0-9.]*\).*/\1/p')
    TP=$(echo "$LINE" | sed -n 's/.*tp=\([0-9.]*\).*/\1/p')
    FREQ=$(echo "$LINE" | sed -n 's/.*freq=\([0-9.]*\).*/\1/p')
    CLOCKS=$(echo "$LINE" | sed -n 's/.* \([0-9.]*\) clocks\/step.*/\1/p')

    GAS=${GAS_USED[$BLOCK]}

    echo "| $BLOCK | $GAS | $STEPS | $DURATION | $TP | $FREQ | $CLOCKS |"
done
