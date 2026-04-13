# ADR-0013: manga のページ順は `manga/pages/` の辞書順を既定にする

- Status: Accepted
- Date: 2026-04-13

## Context

`manga` は prose とは別の原稿モデルを持つが、v0.1 時点では page manifest schema の詳細をまだ固定していない。

一方で、`build` と `validate` を実装するには、manga の入力ページ列を決定する規則が先に必要になる。

現在の `init` scaffold は次を生成する。

- `manga/pages/`
- `manga/spreads/`
- `manga/metadata/`

このため、manifest を待たずに最小の deterministic な入力解決規則を持たせる必要がある。

## Decision

v0.1 の `manga build` / `manga validate` では、明示 manifest が未定義の間、`manga/pages/` 配下の PNG / JPEG ファイルを辞書順で解決してページ順とみなす。

ルール:

- 対象は `manga/pages/` 直下の PNG / JPEG ファイル
- ファイル名の辞書順をそのままページ順に使う
- `manga/pages/` が無い、または対象画像が 1 枚も無い場合は preflight error
- 非画像ファイルは v0.1 では build 入力から無視する
- explicit な spread metadata が未定義の間、Kindle 向けの `spread_policy_for_kindle` は横長ページを見開き候補として扱う
- `split` は横長ページを 2 分割し、`book.reading_direction` に従って Kindle のページ順へ並べる
- `single-page` は横長ページを 1 ページのまま残す
- `skip` は横長ページを Kindle 出力から除外し、結果が 0 ページになる場合は error とする

将来 page manifest schema が定まった場合は、その manifest を優先し、この fallback は implicit mode として残すか再評価する。

## Consequences

- manga の `build` / `validate` が deterministic に動く
- 利用者は `001.png`, `002.png` のような命名で順序を管理できる
- v0.1 では spread metadata や guided view metadata を成果物生成の必須入力にはしない
- Kindle 向けの見開き劣化は heuristic になるため、wide だが spread ではないページは将来 explicit metadata で上書きできる余地を残す
- 将来 manifest 導入時に、implicit な辞書順解決との優先順位を再度整理する必要がある
