# ADR-0019: 物語補助は explicit な repo-native scaffold から始める

- Status: Accepted
- Date: 2026-04-13

## Context

`shosei` は日本語出版向けの CLI であり、`Git first`、`single-book` / `series`、repo-relative path、`book.yml` / `series.yml` を軸にしている。

一方で、創作支援の需要としては次がある。

- キャラクター、用語、場所、組織などの管理
- 巻をまたぐ設定の共有
- scene 単位のメモや構造整理

ただし既存仕様では、次も明確に決まっている。

- WYSIWYG エディタは非目標
- 本文を書き換える自動生成は core CLI の中心責務ではない
- `series` では shared data と巻固有 data を分ける

この状態でいきなり story schema 全体や AI 補助まで固定すると、設定 schema と repo scaffold の両方を同時に重くしてしまう。

## Decision

物語補助は、まず explicit な `shosei story scaffold` から始める。

ルール:

- 初期段階の story support は manual-first とする
- `init` の既定 scaffold にはまだ story directory を自動追加しない
- 利用者が必要になった時点で `shosei story scaffold` を明示実行して story workspace を作る
- `single-book` では `story/` を作る
- `series` では 2 層を区別する
  - 共有 canon: `shared/metadata/story/`
  - 巻固有 story data: `books/<book-id>/story/`
- 初期段階では config field を増やさず、repo-native file layout を source of truth にする
- 初回コマンドは scaffold のみに絞り、scene map や continuity lint は後続段階で追加する

## Consequences

- 既存の `init` を重くせずに story support を導入できる
- `single-book` / `series` の repository model に沿った置き場所を先に固定できる
- 将来 `story map`, `story check`, shared canon sync を追加する土台を作れる
- story data を Markdown / YAML / Git diff に乗せたまま扱える
- story schema と AI workflow は、実運用を見ながら段階的に追加できる
