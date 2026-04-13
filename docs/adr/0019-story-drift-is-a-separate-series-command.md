# ADR-0019: story drift は series 向けの別コマンドとして扱う

- Status: Accepted
- Date: 2026-04-14

## Context

`shosei` の story support は manual-first であり、`series` では次の 2 つの保存場所を明示的に分けている。

- shared canon: `shared/metadata/story/`
- 巻固有 story data: `books/<book-id>/story/`

同時に、`story check` は scene index と scene/entity frontmatter の軽い整合チェックを担う方向で育てている。

この状態で shared canon と巻固有 data のズレまで `story check` に混ぜると、scene 参照整合と canon 衝突検査の責務が曖昧になる。

## Decision

shared canon と巻固有 story data の衝突検査は、`story check` ではなく `story drift` という別コマンドで扱う。

ルール:

- `story check` は `scenes.yml`、scene frontmatter、entity frontmatter の整合を扱う
- `story drift` は `series` のみを対象にする
- `story drift` は `shared/metadata/story/` と `books/<book-id>/story/` の entity Markdown を比較する
- same-scope duplicate entity `id` は error とする
- shared/book で同じ kind と `id` を持ち、内容が分岐していれば drift error とする
- shared/book で同じ kind と `id` を持ち、内容が同じなら redundant copy warning とする

## Consequences

- `story check` は scene 整合の入口として軽いまま保てる
- shared canon の衝突は `series` 専用の明示コマンドで扱える
- 将来 `story sync` や shared canon apply のような操作系を追加しやすくなる
