# ADR-0028: 縦組み prose print は Chromium backend を使う

## Status

Accepted

## Context

ADR-0026 では prose print の v0.1 backend を `weasyprint` に統一した。

その後、`novel` / `light-novel` の default design を実際の build に通し、生成物を確認したところ、縦組み CSS を渡しても print PDF は横組みのままだった。現行の WeasyPrint 経路では `writing-mode: vertical-rl` が組版に反映されず、`vertical-rl` prose の既定 profile を満たせない。

一方で、Pandoc から self-contained HTML を出し、headless Chromium の `--print-to-pdf` で PDF 化すると、同じ CSS の `writing-mode: vertical-rl` が縦組みとして出力されることを確認できた。

v0.1 でも `novel` / `light-novel` の print を「縦組みが出る」状態に保つ必要がある。

## Decision

v0.1 の prose print backend は writing mode / profile ごとの support matrix に切り替える。

- `pdf.engine` に `chromium` を追加する
- prose print の既定 engine は次のように決める
  - `book.writing_mode: vertical-rl` なら `chromium`
  - `book.writing_mode: horizontal-ltr` なら `weasyprint`
  - `book.profile: conference-preprint` は `weasyprint`
- `chromium` backend では
  1. Pandoc で `base.css`, `print.css`, generated layout stylesheet を含む self-contained HTML を生成する
  2. headless Chromium で `--print-to-pdf` し、print PDF を作る
- `weasyprint` backend は horizontal prose と `conference-preprint` の正式経路として残す
- `book.writing_mode: vertical-rl` かつ `pdf.engine: weasyprint` は validate / build で error にする
- `doctor` の required tool は `git`, `pandoc`, `weasyprint`, `chromium` に更新する
- `typst`, `lualatex` は config 値としては引き続き受け付けるが、v0.1 の正式既定 backend にはしない

## Consequences

- `novel` / `light-novel` の print build には Chromium 系ブラウザが必要になる
- `business`, `paper`, `conference-preprint` は従来どおり Pandoc + `weasyprint` を中心に扱える
- `init`, config schema, usage, doctor, validation を writing mode ベースの support matrix に合わせて揃える必要がある
- ADR-0026 の「prose print backend を 1 つに固定する」という前提は更新される。v0.1 の正式 support は `weasyprint` と `chromium` の 2 系統になる
