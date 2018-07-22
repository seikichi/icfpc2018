# How to use

公式のシミュレーターを利用しスコアを計算します。
コマンド実行時の引数に assemble, disassemble, reassemble のいずれかを指定して下さい。

Docker コンテナ内に以下のファイルがあることを仮定しています。

- `/app/source.mdl` (assemle 時不要)
- `/app/target.mdl` (disassemble 時不要)
- `/app/trace.nbt`

適宜 `-v` オプションでマウントして下さい。なお `-v /path/to/dir:/app` などと
`/app` 自体に対してボリュームマウントするとプログラムが正常に動作しません。
これは [alekzonder/puppeteer](https://github.com/alekzonder/docker-puppeteer) の仕様です。

## Assemble

```bash
> docker run --shm-size 1G --rm \
    -v "path/to/current/directory/index.js:/app/index.js" \
    -v "path/to/file.mdl:/app/target.mdl" \
    -v "path/to/file.nbt:/app/trace.nbt" \
    alekzonder/puppeteer:latest node index.js assemble
```

## Assemble

```bash
> docker run --shm-size 1G --rm \
    -v "path/to/current/directory/index.js:/app/index.js" \
    -v "path/to/file.mdl:/app/source.mdl" \
    -v "path/to/file.nbt:/app/trace.nbt" \
    alekzonder/puppeteer:latest node index.js disassemble
```

## Assemble

```bash
> docker run --shm-size 1G --rm \
    -v "path/to/current/directory/index.js:/app/index.js" \
    -v "path/to/src.mdl:/app/source.mdl" \
    -v "path/to/tgt.mdl:/app/target.mdl" \
    -v "path/to/file.nbt:/app/trace.nbt" \
    alekzonder/puppeteer:latest node index.js reassemble
```


