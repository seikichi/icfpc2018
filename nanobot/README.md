# How to use

## nanobot

面倒な人は先に以下を実行。

```sh
export GOLD_AI=default GOLD_ASSEMBLER=default GOLD_DISASSEMBLER=default
```

`GOLD_AI=default` の場合 reassemble の問題は `NaiveReassembleAI` を利用します。
この AI は既存の AssembleAI と DisassembleAI を組合せて利用します。それぞれの AI を
`GOLD_ASSEMBLER` および `GOLD_DISASSEMBLER` で指定します。

補足: GOLD_AI=default の場合

- assemble: GridFissionAI
- disassemble: VoidAI
- reassemble: NaiveReassembleAI

```sh
$ cargo run --release --bin nanobot assemble model.mdl trace.nbt
```

## score

```sh
$ cargo run --release --bin score -- --trace dfltTracesF\FR115.nbt --source problemsF\FR115_tgt.mdl --target problemsF\FR115_tgt.mdl
```
