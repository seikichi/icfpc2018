#!/bin/bash

set -eux

PROBLEM=$1
OUTPUT=$2

cd nanobot

for i in {001..186}; do
    cargo run --release assemble ${PROBLEM}/FA${i}_tgt.mdl ${OUTPUT}/FA${i}.nbt;
done

for i in {001..186}; do
    cargo run --release disassemble ${PROBLEM}/FD${i}_src.mdl ${OUTPUT}/FD${i}.nbt;
done

for i in {001..115}; do
    cargo run --release reassemble ${PROBLEM}/FR${i}_{src,tgt}.mdl ${OUTPUT}/FR${i}.nbt;
done
