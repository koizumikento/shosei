# ADR-0025: prose print の v0.1 PDF backend は typst を正式採用する

- Status: Accepted
- Date: 2026-04-14

## Context

prose print は Pandoc を中核にしつつ、`pdf.engine` として `weasyprint`, `typst`, `lualatex` を受け付ける前提で仕様を整理してきた。

一方で、v0.1 の support matrix を決めないまま doctor と usage を広げたため、次の混乱が起きた。

- どの PDF backend を v0.1 の標準構成としてインストールすべきか不明確
- doctor が `weasyprint`, `typst`, `lualatex`, generic な `PDF engine` を並列に扱い、必須依存に見えてしまう
- scaffold / build / validate / editor integration で、既定 backend の説明が一貫しない

Pandoc 自体は変換の中核だが、print PDF を実際に出力するには別 backend が必要である。v0.1 では、その backend を 1 つに絞って support matrix を明示する必要がある。

## Decision

v0.1 の prose print PDF backend は `typst` を正式採用する。

具体的には次の方針とする。

- `pdf.engine` の既定値は `typst`
- prose print build では Pandoc 実行時に `--pdf-engine typst` を明示する
- `doctor` の required tool は `git`, `pandoc`, `typst`
- `doctor` の optional tool は `epubcheck`, `git-lfs`, Kindle Previewer
- `weasyprint`, `lualatex` は config 値としては受け付けるが、v0.1 の doctor / CI の必須サポート対象には含めない

## Consequences

- `init` scaffold、usage、VS Code extension 表示、preflight の説明を `typst` 既定に揃える必要がある
- `doctor --json` は required / optional の分類を返し、editor はその区分をそのまま表示できる
- `weasyprint`, `lualatex` を v0.1 の正式 backend に昇格させる場合は、別 ADR で support matrix と導入方針を更新する
