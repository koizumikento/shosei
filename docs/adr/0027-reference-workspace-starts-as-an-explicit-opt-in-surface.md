# ADR-0027: 参考資料ワークスペースは独立した opt-in surface として導入する

- Status: Accepted
- Date: 2026-04-14

## Context

`shosei` では、執筆や編集の途中で参照したい URL、調査メモ、判断メモを repo 内に残したい需要がある。

この種の情報は prose / manga / paper をまたいで発生しうる一方、既存 surface にはすでに明確な責務がある。

- `editorial`
  - prose 系の review readiness を扱う
  - claim / figure / freshness などの提出前確認に寄る
- `story`
  - 物語補助のための canon / codex / scene 補助を扱う

ここで参考リンクや汎用メモを `editorial` に寄せると prose 固有の surface が過積載になり、`story` に寄せると物語用途以外との境界が曖昧になる。

また、`shosei` は WYSIWYG や bookmark sync ではなく、repo-native file layout と manual-first な制作フローを重視している。

## Decision

参考リンクと作業メモは、`editorial` / `story` とは別の `reference` surface として導入する。

ルール:

- 対象は全 `project.type` とする
- 初期段階の reference support は manual-first とする
- `init` の既定 scaffold には reference directory を自動追加しない
- 利用者が必要になった時点で `shosei reference scaffold` を明示実行して workspace を作る
- `single-book` では `references/` を作る
- `series` では 2 層を区別する
  - 共有 reference: `shared/metadata/references/`
  - 巻固有 reference: `books/<book-id>/references/`
- 初期段階では config field を増やさず、repo-native file layout を source of truth にする
- 初回コマンドは `reference scaffold` のみに絞り、一覧や検査は後続段階で追加する
- reference entry は Markdown 1 file を基本とし、必要な structured field は frontmatter に置く

## Consequences

- prose 固有の `editorial` と物語固有の `story` の責務を濁さずに済む
- `single-book` / `series` の repository model に沿った保存場所を先に固定できる
- Markdown / YAML / Git diff に乗る形で参考資料を管理できる
- 将来 `reference map` や `reference check` を追加する土台を作れる
- link validation、shared/book 間の drift、`editorial.claims.yml` との相互参照は実運用を見ながら段階的に追加できる
