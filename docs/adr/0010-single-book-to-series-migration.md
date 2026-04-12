# ADR-0010: single-book から series への移行パスを前提にする

- Status: Accepted
- Date: 2026-04-12

## Context

初期状態では単発作品として始めても、後から続刊やシリーズ化が起きる可能性がある。

特に小説、ライトノベル、漫画では次の状況が現実的だった。

- 1 冊目の後でシリーズ化が決まる
- 共通 styles や fonts を巻またぎで管理したくなる
- 世界観資料やキャラ資料を shared 化したくなる

`single-book` と `series` を別モデルとして定義しただけでは、既存 repo の移行パスがない。

## Decision

`single-book -> series` の移行を正式に考慮する。

方針:

- 将来 `shosei migrate --to series --book-id <id>` を提供する
- 移行は Git 履歴を rewrite せず、rename ベースで行う
- 自動共通化は保守的に行い、判断が割れる項目は巻側に残す

## Consequences

- `single-book` で始めてもシリーズ化しやすくなる
- root 配置と path 設計は移行を意識しておく必要がある
- `series.yml` の defaults と巻固有 `book.yml` の切り分けが重要になる
- `series -> single-book` の逆変換は v0.1 対象外のまま残る
