#!/bin/bash

set -eux

PROBLEM=$1
TRACE=$2
OUTPUT_DIR=$3

for i in {001..186}; do
  (cd nanobot; cargo run --bin score --release -- --json --target ${PROBLEM}/FA${i}_tgt.mdl --trace ${TRACE}/FA${i}.nbt) > $OUTPUT_DIR/FA${i}_score.json
  docker run --shm-size 1G --rm \
    -v "$(pwd)/official-score/index.js:/app/index.js" \
    -v "$(pwd)/tmp/FA${i}_tgt.mdl:/app/target.mdl" \
    -v "$(pwd)/tmp/FA${i}.nbt:/app/trace.nbt" \
    alekzonder/puppeteer:latest node index.js assemble > $OUTPUT_DIR/FA${i}_official_score.json
done

for i in {001..186}; do
  cargo run --bin score --release -- --source ${PROBLEM}/FD${i}_src.mdl --trace ${TRACE}/FD${i}.nbt > $OUTPUT_DIR/FD${i}_score.json
  docker run --shm-size 1G --rm \
    -v "$(pwd)/official-score/index.js:/app/index.js" \
    -v "$(pwd)/tmp/FD${i}_src.mdl:/app/source.mdl" \
    -v "$(pwd)/tmp/FD${i}.nbt:/app/trace.nbt" \
    alekzonder/puppeteer:latest node index.js disassemble > $OUTPUT_DIR/FD${i}_official_score.json
done

for i in {001..115}; do
  cargo run --bin score --release -- --source ${PROBLEM}/FR${i}_src.mdl --target ${PROBLEM}/FR${i}_tgt.mdl --trace ${TRACE}/FR${i}.nbt > $OUTPUT_DIR/FR${i}_score.json
  docker run --shm-size 1G --rm \
    -v "$(pwd)/official-score/index.js:/app/index.js" \
    -v "$(pwd)/tmp/FR${i}_src.mdl:/app/source.mdl" \
    -v "$(pwd)/tmp/FR${i}_tgt.mdl:/app/target.mdl" \
    -v "$(pwd)/tmp/FR${i}.nbt:/app/trace.nbt" \
    alekzonder/puppeteer:latest node index.js reassemble > $OUTPUT_DIR/FR${i}_official_score.json
done
