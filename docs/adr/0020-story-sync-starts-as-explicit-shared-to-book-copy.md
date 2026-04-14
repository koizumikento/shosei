# ADR-0020: story sync は explicit な shared-to-book copy から始める

- Status: Superseded
- Date: 2026-04-14

## Context

`story drift` を分離したことで、`series` では shared canon と巻固有 story data の衝突を report できるようになった。

ただし report だけでは、shared canon を正として巻側へ取り込みたいケースを毎回手動 copy に頼ることになる。

一方で、双方向 sync や自動 merge をいきなり入れると、shared と book の優先順位や conflict 解消方針まで同時に固定する必要がある。

## Decision

最初の `story sync` は、shared canon の 1 entity を巻固有 story workspace へ明示コピーする one-way command とする。

ルール:

- 対象は `series` のみ
- source は `shared` のみ受け付ける
- 対象 entity は `kind` と `id` で 1 件ずつ指定する
- book 側に同じ `id` が無い場合は copy する
- book 側に同じ `id` があり内容も同じ場合は no-op とする
- book 側に同じ `id` があり内容が違う場合は、`--force` が無い限り上書きしない
- shared 側の更新や automatic merge は行わない

## Consequences

- drift report の次に取る最小の修復操作を CLI で提供できる
- shared canon を正として巻固有 story data を揃える作業を安全に反復できる
- 将来 `--to shared`、複数 entity の一括同期、merge 戦略を追加する余地を残せる
