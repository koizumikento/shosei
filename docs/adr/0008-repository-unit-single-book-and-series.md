# ADR-0008: リポジトリ管理単位は single-book と series を正式サポートする

- Status: Accepted
- Date: 2026-04-12

## Context

作品をどの単位で Git リポジトリに載せるかが未確定だった。

単発作品では 1 冊 1 repo が自然だが、シリーズ作品では以下の共有要素が多い。

- 共通 styles
- 共通 fonts
- 共通画像資産
- 世界観資料、用語集、キャラクター資料
- 共通 validation policy
- 共通 CI 設定

特にライトノベルや漫画では、巻ごとの差分よりもシリーズ内共有資産の比重が大きい。

## Decision

正式サポートする repository model は次の 2 つとする。

- `single-book`
- `series`

運用方針:

- 単発作品は `single-book`
- シリーズ作品は `series`
- 無関係な複数シリーズを 1 repo に混在させない

設定ファイル:

- `single-book`: root に `book.yml`
- `series`: root に `series.yml`, 各巻に `books/<book-id>/book.yml`

## Consequences

- CLI は root に `book.yml` があるか `series.yml` があるかで動作モードを切り替える必要がある
- `series.yml` と巻固有 `book.yml` の継承ルールが必要になる
- `shosei init` で `repo_mode` を選ばせる必要がある
- `multi-series` を対象外にすることで v0.1 の複雑さを抑えられる
