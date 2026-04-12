# ADR-0009: 設定探索は上方探索、継承優先順位は CLI > book.yml > series.yml > profile defaults とする

- Status: Accepted
- Date: 2026-04-12

## Context

`single-book` と `series` を両方扱う以上、CLI がどこから repo root を見つけ、どの設定をどの順で適用するかを先に決めておく必要があった。

未決定だと次が曖昧になる。

- `books/vol-01` 配下で実行したときの対象巻
- `series.yml` と `book.yml` のどちらが強いか
- array を merge するのか置換するのか
- 共有 assets を config merge とみなすか resource path とみなすか

## Decision

次を採用する。

### 探索

- repo root は現在位置または `--path` から親方向へ上方探索する
- 同一ディレクトリに `book.yml` と `series.yml` が共存する構成は v0.1 では error

### 対象 book

- `series` では `--book` が最優先
- それがなければ `books/<book-id>/` 配下にいる場合だけ自動解決
- `series` root で巻未指定の build/validate/handoff は error

### 優先順位

1. CLI override
2. 巻固有 `book.yml`
3. `series.yml`
4. profile defaults

### merge

- scalar: 後勝ち
- object: 再帰 merge
- array: 基本置換
- `shared/*` は merge ではなく resource resolution 規則として扱う

## Consequences

- 実装は比較的単純で予測可能になる
- `series` root での暗黙 build を避けられる
- array append を期待するユーザーには明示が必要になる
- 設定 merge より resource 探索の設計が重要になる
