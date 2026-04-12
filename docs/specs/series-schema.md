# `series.yml` schema v0.1

作成日: 2026-04-12  
状態: Draft

## 1. 目的

この文書は、シリーズ単位リポジトリの root 設定である `series.yml` の最小 schema を定義する。

`series.yml` は共通設定を持ち、各巻の `books/<book-id>/book.yml` に既定値を提供する。

## 2. 方針

- `series.yml` は `repo_mode = series` のときのみ使用する
- 作品固有ではなく、シリーズ共通情報だけを置く
- パスは repo root 基準の相対パスで表現する

## 3. ルート構造

```yaml
series:
shared:
defaults:
validation:
git:
books:
```

## 4. `series`

```yaml
series:
  id: my-series
  title: "シリーズ名"
  language: ja
  type: light-novel
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `id` | string | yes | none | slug |
| `title` | string | yes | none | non-empty |
| `language` | string | no | `ja` | BCP 47 compatible string |
| `type` | string | yes | none | `business`, `novel`, `light-novel`, `manga` |

## 5. `shared`

```yaml
shared:
  assets:
    - shared/assets
  styles:
    - shared/styles
  fonts:
    - shared/fonts
  metadata:
    - shared/metadata
```

| Field | Type | Required | Default |
|---|---|---|---|
| `assets` | array<string> | no | `[]` |
| `styles` | array<string> | no | `[]` |
| `fonts` | array<string> | no | `[]` |
| `metadata` | array<string> | no | `[]` |

## 6. `defaults`

```yaml
defaults:
  book:
    profile: light-novel
    writing_mode: vertical-rl
    reading_direction: rtl
  layout:
    binding: right
    chapter_start_page: odd
    allow_blank_pages: true
  outputs:
    kindle:
      enabled: true
      target: kindle-ja
```

| Field | Type | Required | Default |
|---|---|---|---|
| `book` | object | no | `{}` |
| `layout` | object | no | `{}` |
| `outputs` | object | no | `{}` |
| `pdf` | object | no | `{}` |
| `print` | object | no | `{}` |
| `images` | object | no | `{}` |
| `manga` | object | no | `{}` |

制約:

- 各オブジェクトの中身は `book.yml` schema と同じ意味を持つ
- 巻固有 `book.yml` が存在する場合は、そちらが優先される

## 7. `validation`

```yaml
validation:
  strict: true
  epubcheck: true
  accessibility: warn
```

| Field | Type | Required | Default |
|---|---|---|---|
| `strict` | boolean | no | `true` |
| `epubcheck` | boolean | no | `true` |
| `accessibility` | string | no | `warn` |

## 8. `git`

```yaml
git:
  lfs: true
  require_clean_worktree_for_handoff: true
```

| Field | Type | Required | Default |
|---|---|---|---|
| `lfs` | boolean | no | `true` |
| `require_clean_worktree_for_handoff` | boolean | no | `true` |

## 9. `books`

```yaml
books:
  - id: vol-01
    path: books/vol-01
    number: 1
    title: "第1巻"
  - id: vol-02
    path: books/vol-02
    number: 2
    title: "第2巻"
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `id` | string | yes | none | slug |
| `path` | string | yes | none | repo-relative path |
| `number` | integer | no | none | positive integer |
| `title` | string | no | none | any |

制約:

- `path` 先には `book.yml` が存在すること
- `id` は series 内で一意

## 10. 最小例

```yaml
series:
  id: sample-series
  title: "サンプルシリーズ"
  language: ja
  type: manga

shared:
  assets:
    - shared/assets
  styles:
    - shared/styles

defaults:
  book:
    profile: manga
    writing_mode: vertical-rl
    reading_direction: rtl
  layout:
    binding: right
  outputs:
    kindle:
      enabled: true
      target: kindle-comic
    print:
      enabled: true
      target: print-manga

git:
  lfs: true
  require_clean_worktree_for_handoff: true

books:
  - id: vol-01
    path: books/vol-01
    number: 1
    title: "第1巻"
```

## 11. v0.1 の制約

- `series.yml` は 1 repo に 1 つだけ
- `multi-series` は対象外
- `books` 配下のネストは 1 段までを想定
