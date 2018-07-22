#!/bin/bash

set -eux

PROBLEM=$1
TRACE=$2

cd nanobot

for i in {001..186}; do
    cargo run --bin score --release -- --target ${PROBLEM}/FA${i}_tgt.mdl --trace ${TRACE}/FA${i}.nbt;
done

for i in {001..186}; do
    cargo run --bin score --release -- --source ${PROBLEM}/FD${i}_src.mdl --trace ${TRACE}/FD${i}.nbt;
done

for i in {001..115}; do
    cargo run --bin score --release -- --source ${PROBLEM}/FR${i}_src.mdl --target ${PROBLEM}/FR${i}_tgt.mdl --trace ${TRACE}/FR${i}.nbt;
done
