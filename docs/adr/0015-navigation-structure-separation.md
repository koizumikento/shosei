# ADR-0015: navigation 構造は source / semantic / navigation で分離する

- Status: Accepted
- Date: 2026-04-13

## Context

現状の仕様には次が存在する。

- `manuscript.frontmatter`, `manuscript.chapters`, `manuscript.backmatter` による prose のファイル順
- `sections.type` によるファイルの意味分類
- `pdf.toc`, `pdf.running_header`, EPUB nav など見出し利用を前提にする出力要件
- `manga/pages/` を primary source にする漫画向け build graph

一方で、章題や節題をどこから取得するか、どの profile でどの深さまで navigation に使うか、`sections` が見出し文字列まで持つのかが未整理だった。

このままだと、prose と manga の両方で同じ見出しモデルを強制するか、逆に出力機能ごとに別々の暗黙ルールを持つことになる。

## Decision

v0.1 では構造情報を次の 3 層に分けて扱う。

- source structure
  - ファイル順、またはページ順
- semantic structure
  - `titlepage`, `chapter`, `appendix`, `afterword`, `colophon` などの意味分類
- navigation structure
  - 目次、EPUB nav、PDF bookmark、running header に使う見出し階層

追加ルール:

- `sections` は semantic structure のための schema とし、章題や節題の文字列は持たせない
- prose の source structure における章順は `manuscript.chapters` の配列順を正とする
- prose の filename prefix は source structure の正典ではない
- prose では Markdown 見出しを navigation structure の source of truth にする
- `manuscript.chapters` に含まれる各本文ファイルの最初の level-1 heading を chapter title として扱う
- level-2 以降の heading は section / subsection 候補として扱う
- `business` は chapter と section の両方を navigation に使いやすくする
- `novel` と `light-novel` は chapter 中心を既定にする
- manga は page order を primary source にし、v0.1 では chapter title や section title を必須にしない
- manga の nav は既定で page sequence を使い、chapter/episode metadata は将来拡張として追加する
- v0.1 では `book.yml` に `title`, `short_title`, `include_in_toc` など navigation override field を追加しない

## Consequences

- prose の TOC、EPUB nav、PDF bookmark、running header の source が明確になる
- `sections` の責務が広がりすぎず、file semantics に集中できる
- `business`, `novel`, `light-novel`, `manga` で異なる navigation 深さや必須度を表現しやすい
- manga でも章扉や各話タイトルの存在を否定せず、page-based model を維持できる
- 将来は navigation override、short title、chapter/episode metadata を別拡張として追加できる
