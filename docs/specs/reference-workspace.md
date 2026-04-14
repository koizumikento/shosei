# 参考資料ワークスペース仕様 v0.1

作成日: 2026-04-14  
状態: Draft

## 1. 目的

この文書は、`shosei` における参考リンクと作業メモの最小仕様を定義する。

最初の対象は次の通り。

- 執筆・編集時に参照する外部リンクや資料メモを repo-native に保持する
- `single-book` と `series` に沿った保存場所を固定する
- Markdown / YAML / Git diff に乗る manual-first な運用に留める

## 2. 非目標

- ブラウザ bookmark sync
- Web ページ本文の自動保存や全文ミラー
- 本文への自動 citation 挿入
- `book.yml` / `series.yml` への reference 設定の大量追加
- `story` / `editorial` の責務の置き換え

## 3. 位置づけ

- `editorial` は prose 系の review readiness を扱う sidecar であり、claim / figure / freshness の管理を担う
- `story` は物語補助のための worldbuilding / scene / codex を扱う opt-in workspace である
- reference workspace はジャンル非依存で、執筆中に参照する外部情報と判断メモの置き場を担う

設計方針:

- 最初は explicit command で opt-in にする
- source of truth は repo 内ファイルとする
- `single-book` と `series` の差は保存場所だけに留める
- shared で再利用したい参考資料と、book ごとの作業メモを混ぜない
- path を保持する場合は repo-relative かつ `/` 区切り前提で運用する

## 4. ディレクトリ規約

### 4.1 `single-book`

```text
repo/
  references/
    README.md
    entries/
      README.md
```

### 4.2 `series`

共有:

```text
repo/
  shared/
    metadata/
      references/
        README.md
        entries/
          README.md
```

巻固有:

```text
repo/
  books/
    vol-01/
      references/
        README.md
        entries/
          README.md
```

ルール:

- `single-book` では root の `references/` を使う
- `series` では共通で再利用したい資料を `shared/metadata/references/` に置く
- `series` では巻固有の調査メモや取材リンクを `books/<book-id>/references/` に置く
- shared 側と巻固有側で同じ topic を持ってもよいが、source of truth は file 単位で明示する

## 5. `shosei reference scaffold`

reference workspace を生成する。

### 5.1 コマンド形

```bash
shosei reference scaffold
shosei reference scaffold --book vol-01
shosei reference scaffold --shared
shosei reference scaffold --force
```

### 5.2 振る舞い

- `single-book`
  - `shosei reference scaffold` は `references/` を生成する
  - `--shared` は error
- `series`
  - `shosei reference scaffold --shared` は `shared/metadata/references/` を生成する
  - `shosei reference scaffold` は対象 book の `books/<book-id>/references/` を生成する
  - repo root から巻固有 scaffold を作る場合は `--book <book-id>` を要求する
  - `books/<book-id>/...` の内側で実行した場合は対象 book を推定できる
- 既存 file は既定で保持する
- `--force` を付けた場合だけ template file を上書きする
- `book.yml` / `series.yml` は更新しない

### 5.3 生成物

- `README.md`
- `entries/README.md`

## 6. 最小データ形

v0.1 では、参考資料 1 件を Markdown 1 file で表す。

例:

```md
---
id: market-report-2026
title: 2026年国内市場レポート
links:
  - https://example.com/report
tags:
  - market
  - source
related_sections:
  - manuscript/02-market.md
status: unread
---

- 要点
- 気になった前提
- 本文へ反映するかの判断メモ
```

ルール:

- 1 file 1 entry とする
- file は `entries/<slug>.md` を推奨する
- `id` は scope 内で一意な stable identifier とする
- `title` は人間向けの見出し
- `links` は 0 件以上を許容し、外部 URL または repo-relative path を置ける
- `tags` は任意の分類ラベル
- `related_sections` は任意で、関連する manuscript path を置ける
- `status` は当面 `unread | reading | summarized | applied` を想定するが、未知の値も将来拡張のため許容する
- 本文は自由記述の Markdown メモとする

## 7. `shosei reference map`

reference entry 一覧を読み、text と JSON report を出力する。

### 7.1 コマンド形

```bash
shosei reference map
shosei reference map --book vol-01
shosei reference map --shared
```

### 7.2 振る舞い

- `single-book`
  - `shosei reference map` は `references/entries/` を読む
  - `--shared` は error
- `series`
  - `shosei reference map --shared` は `shared/metadata/references/entries/` を読む
  - `shosei reference map` は対象 book の `books/<book-id>/references/entries/` を読む
  - repo root から巻固有 map を実行する場合は `--book <book-id>` を要求する
  - `books/<book-id>/...` の内側で実行した場合は対象 book を推定できる
- 対象は `entries/` 直下の `*.md` のみとし、`README.md` は対象外
- report は次に出力する
  - `single-book`: `dist/reports/default-reference-map.json`
  - `series --book <book-id>`: `dist/reports/<book-id>-reference-map.json`
  - `series --shared`: `dist/reports/shared-reference-map.json`

### 7.3 最小出力

- entry 数
- 各 entry の file path
- 各 entry の `id`
- optional `title`
- `links` 件数
- optional `status`

- `id` は frontmatter の `id` を優先し、未指定時は filename stem を使う
- frontmatter がある場合は YAML mapping とする
- `title`, `status` は string
- `links`, `tags`, `related_sections` は string 配列

## 8. `shosei reference check`

reference entry を検査し、issue report を出力する。

### 8.1 コマンド形

```bash
shosei reference check
shosei reference check --book vol-01
shosei reference check --shared
```

### 8.2 振る舞い

- `single-book`
  - `shosei reference check` は `references/entries/` を読む
  - `--shared` は error
- `series`
  - `shosei reference check --shared` は `shared/metadata/references/entries/` を読む
  - `shosei reference check` は対象 book の `books/<book-id>/references/entries/` を読む
  - repo root から巻固有 check を実行する場合は `--book <book-id>` を要求する
  - `books/<book-id>/...` の内側で実行した場合は対象 book を推定できる
- 対象は `entries/` 直下の `*.md` のみとし、`README.md` は対象外
- report は次に出力する
  - `single-book`: `dist/reports/default-reference-check.json`
  - `series --book <book-id>`: `dist/reports/<book-id>-reference-check.json`
  - `series --shared`: `dist/reports/shared-reference-check.json`

### 8.3 v0.1 の検査対象

- invalid frontmatter
  - frontmatter が閉じていない
  - frontmatter root が YAML mapping ではない
  - `title`, `status` が string でない
  - `links`, `tags`, `related_sections` が string 配列でない
- `id`
  - frontmatter の `id` が空文字
  - frontmatter `id` と filename stem のどちらからも `id` を解決できない
  - 同一 scope 内の duplicate `id`
- local path
  - `links` のうち URL ではない値は repo-relative path として解釈する
  - `related_sections` は repo-relative path として解釈する
  - invalid repo-relative path は error
  - path 解決先が存在しない場合は warning
  - `https://`, `http://`, `mailto:`, `tel:`、`#anchor` は存在確認しない
- prose claim source
  - `single-book` と `series` の巻固有 scope では、selected book の config が prose project として解決でき、`editorial.claims` が設定されている場合にだけ `claims.yml` も読む
  - `claims.yml` の `sources` にある `ref:<id>` は reference entry id として扱う
  - `single-book` では `references/entries/` の `id` に解決する
  - `series` の巻固有 scope では `books/<book-id>/references/entries/` と `shared/metadata/references/entries/` の `id` に解決する
  - `ref:` の後ろが空文字なら error
  - 解決先の entry が見つからない `ref:<id>` は error
  - `ref:` 以外の `sources` 値は既存の prose editorial workflow に委ね、この command では解釈しない

### 8.4 非目標

- 外部 URL の到達確認
- 本文中の実際の引用箇所との照合
- shared と巻固有 reference の drift 判定

## 9. `shosei reference drift`

`series` で shared reference と巻固有 reference の衝突と gap を report 化する。

### 9.1 コマンド形

```bash
shosei reference drift --book vol-01
```

### 9.2 振る舞い

- `series` の巻固有 scope 専用とする
- repo root から実行する場合は `--book <book-id>` を要求する
- `books/<book-id>/...` の内側で実行した場合は対象 book を推定できる
- 比較対象は次の 2 つ
  - `shared/metadata/references/entries/`
  - `books/<book-id>/references/entries/`
- report は `dist/reports/<book-id>-reference-drift.json` に出力する
- `entries/` directory が存在しない scope は empty として扱う

### 9.3 v0.1 の比較ルール

- `id` は frontmatter 優先、未指定時は filename stem を使う
- invalid frontmatter や non-empty でない `id` は error issue として report する
- same-scope duplicate `id` は error issue として report する
- shared と book の両方に同じ `id` があるときだけ `drifts` entry を作る
- shared と book の file contents が同一なら `redundant-copy`
- shared と book の file contents が異なるなら `drift`
- `redundant-copy` は warning
- `drift` は error
- shared にだけある `id` は `gaps` 配列へ `shared-only` として出す
- book にだけある `id` は `gaps` 配列へ `book-only` として出す
- `shared-only`, `book-only` は warning

### 9.4 非目標

- local path や外部 URL の再検証

## 10. `shosei reference sync`

`series` で shared reference と巻固有 reference の間を明示コピーする。単体 sync と `reference drift` report を使う batch sync の両方を扱う。

### 10.1 コマンド形

```bash
shosei reference sync --book vol-01 --from shared --id market
shosei reference sync --book vol-01 --to shared --id market
shosei reference sync --book vol-01 --from shared --id market --force
shosei reference sync --book vol-01 --to shared --id market --force
shosei reference sync --book vol-01 --from shared --report dist/reports/vol-01-reference-drift.json --force
shosei reference sync --book vol-01 --to shared --report dist/reports/vol-01-reference-drift.json --force
```

### 10.2 振る舞い

- `series` の巻固有 scope 専用とする
- `--from shared` か `--to shared` のどちらか一方を必須にする
- 単体 mode では `--id <id>` を 1 件指定する
- report mode では `--report <path>` を指定し、`--id` は受け付けない
- report mode は `--force` を必須にする
- `--from shared` は shared 側を source にして `books/<book-id>/references/entries/` へ copy する
- `--to shared` は巻固有側を source にして `shared/metadata/references/entries/` へ copy する
- report mode では `drifts` に加えて source 側に存在する `gaps` も適用対象にする
- `--from shared` は `shared-only` gap を適用し、`book-only` gap は skip する
- `--to shared` は `book-only` gap を適用し、`shared-only` gap は skip する
- destination 側に同じ `id` の entry があり内容が同じなら no-op とする
- destination 側に同じ `id` の entry があり内容が違う場合は、`--force` が無ければ error、`--force` がある場合だけ source 内容で上書きする
- destination 側に同じ `id` が無い場合は source file の filename を引き継いで新規作成する

### 10.3 非目標

- 自動 merge
- `reference drift` を飛ばした自動推測同期
- `links` や `related_sections` の再検証

## 11. v0.1 の最小責務

この段階では、まず次だけを対象にする。

- workspace を明示 command で生成できるようにする
- workspace の置き場所を固定する
- 参考資料 entry の最小 shape を決める
- 参考資料 entry 一覧を report 化できるようにする
- 参考資料 entry の shape / duplicate `id` / local path を軽く検査できるようにする
- prose book では `editorial.claims.yml` の `ref:<id>` source を reference entry と照合できるようにする
- `series` で shared/book 間の衝突を report 化できるようにする
- `series` で shared/book 間の明示 sync を行えるようにする
- `single-book` / `series` の scope 差を保存場所で表現する

まだ対象にしないもの:

- 本文参照との自動照合
- `editorial.claims.yml` との自動相互変換
- VS Code 上での専用 UI

## 12. 今後の拡張候補

- `editorial.claims.yml` の `ref:<id>` 以外の source 記法や `story/` からの相互参照
