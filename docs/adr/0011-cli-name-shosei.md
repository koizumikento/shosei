# ADR-0011: CLI 名は shosei を採用する

- Status: Accepted
- Date: 2026-04-12

## Context

仕様上の仮コマンド名として `book` を使っていたが、正式な配布名としては問題があった。

主な懸念:

- 汎用語すぎて検索性が悪い
- 他ツールや将来のパッケージ名と衝突しやすい
- 日本語出版向けツールとしての文脈が出にくい
- `cargo install` やバイナリ名として弱い

一方で、今回のツールは日本語出版、Kindle、印刷、漫画、シリーズ管理まで含むため、狭すぎない固有名が必要だった。

## Decision

CLI バイナリ名は `shosei` を採用する。

命名方針:

- binary: `shosei`
- Rust crates: `shosei-cli`, `shosei-core`
- コマンド例: `shosei init`, `shosei build`, `shosei validate`

補足:

- 設定ファイル名 `book.yml` と `series.yml` は v0.1 では維持する
- コマンド名の変更と設定ファイル名の変更は切り分ける

## Consequences

- docs 内のコマンド例と crate 名を `shosei` に統一する必要がある
- 将来の Cargo workspace 名や配布名を早い段階で揃えられる
- `book` より説明しやすく、衝突しにくい名前になる
