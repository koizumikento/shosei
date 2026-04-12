# ADR-0002: prose は Pandoc を中核にする

- Status: Accepted
- Date: 2026-04-12

## Context

文章主体の書籍を EPUB/PDF に出力する機能をゼロから実装すると、パッケージ生成、目次、metadata、フォント埋め込み、PDF backend 連携など広い領域を抱えることになる。

一方で、Pandoc は prose 系の build に必要な大部分を既に持っている。

## Decision

`business`, `novel`, `light-novel` は Pandoc を中核変換エンジンとして採用する。

本ツールは以下を自前責務とする。

- プロジェクト初期化
- 設定解決
- profile 管理
- アセット管理
- 検証
- handoff

## Consequences

- prose 系の build は Pandoc の制約に乗る
- CLI の価値は変換器そのものではなく orchestration に寄る
- fixed-layout や漫画のような別原稿モデルは Pandoc 以外の経路が必要になる
