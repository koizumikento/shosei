# ADR-0028: `claims.yml` の reference source は `ref:<id>` で明示する

- Status: Accepted
- Date: 2026-04-14

## Context

reference workspace が入ったことで、prose 系の claim ledger から repo-native な参考資料 entry を再利用したい需要が出る。

ただし `claims.yml` の `sources` は既存の prose workflow で URL や自由な資料メモも受け入れており、bare string をそのまま reference id と解釈すると既存運用と衝突しやすい。

また `series` では、巻固有 reference と shared reference のどちらも claim の根拠になりうる。

## Decision

`claims.yml` の `sources` から reference entry を指すときは、`ref:<id>` の明示 prefix を使う。

ルール:

- `ref:<id>` だけを reference entry 参照として扱う
- `single-book` では `references/entries/` の `id` に解決する
- `series` の巻固有 scope では、book 側 `references/entries/` と shared 側 `shared/metadata/references/entries/` の両方に解決できる
- shared reference workspace 自体の `reference check --shared` は `claims.yml` を読まない
- `ref:` 以外の `sources` 値は既存どおり URL や他の source 表現として残し、この判定では解釈しない
- `ref:<id>` の検証は `shosei reference check` の責務とし、prose book の reference workspace があるときだけ行う

## Consequences

- 既存の `claims.yml` source 記法を壊さずに reference entry を導入できる
- claim ledger と reference workspace の結び方が文字列規約として明確になる
- `series` でも shared / book の reference を 1 つの claim source 記法で扱える
- `validate` の prose editorial lint を全面的に作り替えずに、reference workflow の範囲で相互参照を追加できる
