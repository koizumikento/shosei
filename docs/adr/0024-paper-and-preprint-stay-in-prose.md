# ADR-0024: 論文と発表前刷りは prose 系として扱い、発表前刷りは print layout preset で表す

- Status: Accepted
- Date: 2026-04-14

## Context

新しい `mode` 候補として、論文と発表前刷りを追加したい要求が出た。

ここでいう発表前刷りは、見本 PDF から少なくとも次の特徴を持つ。

- A4
- 横書き
- 2 段組
- 短い配布物
- 余白、段間、本文サイズなどに venue 固有の制約がある

一方で、原稿の source 自体は漫画のような page-image model ではなく、本文、図表、参考文献を持つ prose に近い。

発表前刷りを `manga` のような別原稿モデルにすると、原稿構造、Pandoc 利用、editorial sidecar、`chapter` / `explain` / `validate` の共有導線を不必要に分岐させる。

## Decision

- `paper` を prose 系の `project.type` として追加する
- 一般的な論文は `book.profile: paper` を既定とする
- 発表前刷りは `project.type: paper` の下で `book.profile: conference-preprint` として表す
- 発表前刷り向けの差分は別原稿モデルではなく、`pdf` / `print` の layout 設定で明示する
- 発表前刷り向けに、少なくとも次の設定を schema で表せるようにする
  - A4 などの判型
  - 段組数と段間
  - 余白
  - simplex / duplex
  - ページ上限

## Consequences

- `paper` と `conference-preprint` は `business` / `novel` / `light-novel` と同じ prose build graph に乗る
- `chapter`, `explain`, `validate`, `handoff` は prose 共通の責務を再利用できる
- 発表前刷り特有の制約は、template 名や magic behavior ではなく config に出る
- venue ごとの差分は preset を起点に override しやすくなる
- `pdf` / `print` schema は少し広がるが、原稿モデルの分岐は増えない
