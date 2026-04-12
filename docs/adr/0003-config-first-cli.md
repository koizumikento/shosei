# ADR-0003: 設定ファイル中心の優しい CLI にする

- Status: Accepted
- Date: 2026-04-12

## Context

書籍制作では build ごとに大量の引数を渡す運用は再現性が低く、誤操作もしやすい。今回の要望でも、なるべく引数に依存したくないという条件が明示された。

## Decision

CLI は `book.yml` を中心に設計し、日常操作はゼロ引数または最小引数で動くようにする。

基本コマンド:

- `shosei init`
- `shosei build`
- `shosei validate`
- `shosei preview`
- `shosei doctor`
- `shosei handoff`

`shosei init` は対話式ウィザードを標準とする。

## Consequences

- 設定変更はファイルベースで差分管理しやすくなる
- Git と相性が良い
- CLI 引数は一時 override に限定する必要がある
- effective config 表示機能が欲しくなる
