# `book.yml` 設定ファイル schema v0.1

作成日: 2026-04-12  
状態: Draft

## 1. 目的

この文書は、`book.yml` の正式 schema を定義する。

- 何が必須か
- どの値を許可するか
- 既定値は何か
- prose 系と manga 系でどこが分かれるか

実装では YAML を標準とするが、内部表現は別形式でもよい。

## 2. 方針

- `book.yml` はプロジェクト唯一の主要設定源とする
- CLI 引数は原則 override のみ
- schema は `project.type` を起点に条件分岐する
- 省略時の既定値を明確に持つ
- パス表現は OS に依存しない repo-relative 形式で統一する

## 2.1 パス表現ルール

この schema に現れるパス文字列は、すべてリポジトリルート基準の相対パスとして扱う。

ルール:

- 区切り文字は `/` を使う
- `.` から始まるカレントディレクトリ表現は不要
- 絶対パスは許可しない
- 実装時に Windows でも `/` を受け取り、内部で正規化する
- 生成時も config 上は `/` 表記を維持する

## 3. ルート構造

```yaml
project:
book:
layout:
manuscript:
sections:
outputs:
pdf:
print:
images:
validation:
git:
manga:
pipeline:
```

### ルートキー一覧

| Key | Required | Applies to | Description |
|---|---|---|---|
| `project` | yes | all | プロジェクト種別と VCS 方針 |
| `book` | yes | all | 書誌情報と作品 profile |
| `layout` | yes | all | 綴じ方向、章開始ページなど |
| `manuscript` | conditional | prose | prose 系原稿一覧 |
| `sections` | no | prose | section type や上書き指定 |
| `outputs` | yes | all | 有効な出力 target |
| `pdf` | conditional | prose, print | PDF engine と共通 PDF 設定 |
| `print` | conditional | print, manga-print | 印刷向け設定 |
| `images` | no | all | 画像の既定ルール |
| `validation` | yes | all | 検証ポリシー |
| `git` | yes | all | Git/LFS 設定 |
| `manga` | conditional | manga | 漫画向けページモデル設定 |
| `pipeline` | no | manga | 漫画制作工程設定 |

## 4. `project`

```yaml
project:
  type: light-novel
  vcs: git
  version: 1
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `type` | string | yes | none | `business`, `novel`, `light-novel`, `manga` |
| `vcs` | string | yes | `git` | `git` |
| `version` | integer | no | `1` | positive integer |

備考:

- `project.type` が schema 分岐の起点になる
- `vcs` は現時点では `git` 固定

## 5. `book`

```yaml
book:
  title: "作品名"
  subtitle: null
  authors:
    - "著者名"
  language: ja
  profile: light-novel
  writing_mode: vertical-rl
  reading_direction: rtl
  identifier: auto
  rights: "All rights reserved"
  audience: general
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `title` | string | yes | none | non-empty |
| `subtitle` | string or null | no | `null` | any |
| `authors` | array<string> | yes | none | at least 1 item |
| `language` | string | yes | `ja` | BCP 47 compatible string |
| `profile` | string | yes | derived from `project.type` | `business`, `novel`, `light-novel`, `manga` |
| `writing_mode` | string | yes | profile-based | `horizontal-ltr`, `vertical-rl` |
| `reading_direction` | string | yes | derived from `writing_mode` | `ltr`, `rtl` |
| `identifier` | string | no | `auto` | any or `auto` |
| `rights` | string | no | empty string | any |
| `audience` | string | no | `general` | `general`, `children`, `ya`, `professional` |

制約:

- `profile: manga` は `project.type: manga` のときのみ許可
- `writing_mode: vertical-rl` のとき、`reading_direction: rtl` を推奨

## 6. `layout`

```yaml
layout:
  binding: right
  chapter_start_page: odd
  allow_blank_pages: true
  page_progression: auto
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `binding` | string | yes | derived from `writing_mode` | `right`, `left` |
| `chapter_start_page` | string | no | `any` | `any`, `odd` |
| `allow_blank_pages` | boolean | no | `true` | `true`, `false` |
| `page_progression` | string | no | `auto` | `auto`, `ltr`, `rtl` |

## 7. `manuscript`

`project.type != manga` の場合に必須。

```yaml
manuscript:
  frontmatter:
    - manuscript/00-title.md
  chapters:
    - manuscript/01-chapter-1.md
    - manuscript/02-chapter-2.md
  backmatter:
    - manuscript/99-colophon.md
```

| Field | Type | Required | Default |
|---|---|---|---|
| `frontmatter` | array<string> | no | `[]` |
| `chapters` | array<string> | yes | none |
| `backmatter` | array<string> | no | `[]` |

制約:

- `chapters` は 1 件以上必須
- すべてのファイルは workspace 内の相対パス
- 拡張子は v0.1 では `.md` のみ

## 8. `sections`

`sections` は prose 系で任意。`manuscript` の各ファイルに意味属性を付与する。

```yaml
sections:
  - file: manuscript/00-title.md
    type: titlepage
  - file: manuscript/99-colophon.md
    type: colophon
    writing_mode: horizontal-ltr
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `file` | string | yes | none | file path |
| `type` | string | yes | none | `cover`, `titlepage`, `toc`, `chapter`, `appendix`, `afterword`, `colophon` |
| `writing_mode` | string | no | inherit | `horizontal-ltr`, `vertical-rl` |
| `include_in` | array<string> | no | all enabled outputs | `kindle`, `print` |

制約:

- `file` は `manuscript` に現れるファイルと一致すること

## 9. `outputs`

```yaml
outputs:
  kindle:
    enabled: true
    target: kindle-ja
  print:
    enabled: true
    target: print-jp-pdfx1a
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `kindle.enabled` | boolean | no | `false` | `true`, `false` |
| `kindle.target` | string | conditional | `kindle-ja` | `kindle-ja`, `kindle-comic` |
| `print.enabled` | boolean | no | `false` | `true`, `false` |
| `print.target` | string | conditional | `print-jp-pdfx1a` | `print-jp-pdfx1a`, `print-jp-pdfx4`, `print-manga` |

制約:

- 少なくとも 1 つの出力が `enabled: true`
- `project.type: manga` のとき、`kindle.target` は `kindle-comic` を推奨
- `project.type: manga` のとき、`kindle-ja` は将来互換扱いで、v0.1 では非推奨

## 10. `pdf`

`outputs.print.enabled: true` の場合に推奨。`project.type != manga` で PDF backend を指定する。

```yaml
pdf:
  engine: weasyprint
  toc: true
  page_number: true
  running_header: auto
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `engine` | string | conditional | `weasyprint` | `weasyprint`, `typst`, `lualatex` |
| `toc` | boolean | no | `true` | `true`, `false` |
| `page_number` | boolean | no | `true` | `true`, `false` |
| `running_header` | string | no | `auto` | `auto`, `none`, `title`, `chapter` |

## 11. `print`

```yaml
print:
  trim_size: bunko
  bleed: 3mm
  crop_marks: true
  body_pdf: true
  cover_pdf: false
  pdf_standard: pdfx1a
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `trim_size` | string | conditional | `A5` | `A5`, `B6`, `bunko`, `custom` |
| `bleed` | string | no | `3mm` | CSS length string |
| `crop_marks` | boolean | no | `true` | `true`, `false` |
| `body_pdf` | boolean | no | `true` | `true`, `false` |
| `cover_pdf` | boolean | no | `false` | `true`, `false` |
| `pdf_standard` | string | no | derived from target | `pdfx1a`, `pdfx4` |
| `body_mode` | string | no | `auto` | `auto`, `monochrome`, `color` |

制約:

- `trim_size: custom` の場合は将来 `custom_trim_size` の追加が必要
- `cover_pdf: true` は v0.1 では metadata だけ先に定義し、実装は将来に回してもよい

## 12. `images`

```yaml
images:
  default_caption: optional
  default_alt: required
  spread_policy_for_kindle: split
  default_page_side: either
  min_print_dpi: 300
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `default_caption` | string | no | `optional` | `required`, `optional`, `none` |
| `default_alt` | string | no | `required` | `required`, `optional`, `none` |
| `spread_policy_for_kindle` | string | no | `split` | `split`, `single-page`, `skip` |
| `default_page_side` | string | no | `either` | `left`, `right`, `either` |
| `min_print_dpi` | integer | no | `300` | positive integer |

## 13. `validation`

```yaml
validation:
  strict: true
  epubcheck: true
  accessibility: warn
  missing_image: error
  missing_alt: error
  broken_link: error
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `strict` | boolean | no | `true` | `true`, `false` |
| `epubcheck` | boolean | no | `true` | `true`, `false` |
| `accessibility` | string | no | `warn` | `off`, `warn`, `error` |
| `missing_image` | string | no | `error` | `warn`, `error` |
| `missing_alt` | string | no | `error` | `warn`, `error` |
| `broken_link` | string | no | `error` | `warn`, `error` |

## 14. `git`

```yaml
git:
  lfs: true
  require_clean_worktree_for_handoff: true
  lockable:
    - "*.psd"
    - "*.clip"
    - "*.kra"
    - "*.tif"
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `lfs` | boolean | no | `true` | `true`, `false` |
| `require_clean_worktree_for_handoff` | boolean | no | `true` | `true`, `false` |
| `lockable` | array<string> | no | `[]` | glob list |

## 15. `manga`

`project.type: manga` の場合に必須。

```yaml
manga:
  reading_direction: rtl
  default_page_side: right
  page_width: auto
  page_height: auto
  spread_policy_for_kindle: split
  front_color_pages: 4
  body_mode: monochrome
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `reading_direction` | string | yes | `rtl` | `ltr`, `rtl` |
| `default_page_side` | string | no | `right` | `left`, `right` |
| `page_width` | string | no | `auto` | CSS length string or `auto` |
| `page_height` | string | no | `auto` | CSS length string or `auto` |
| `spread_policy_for_kindle` | string | no | `split` | `split`, `single-page`, `skip` |
| `front_color_pages` | integer | no | `0` | `0+` |
| `body_mode` | string | no | `monochrome` | `monochrome`, `color`, `mixed` |

## 16. `pipeline`

`project.type: manga` の場合に任意。

```yaml
pipeline:
  stages:
    - script
    - storyboard
    - art
    - export
    - validate
    - handoff
```

| Field | Type | Required | Default |
|---|---|---|---|
| `stages` | array<string> | no | implementation-defined ordered list |

## 17. prose の最小例

```yaml
project:
  type: novel
  vcs: git

book:
  title: "サンプル小説"
  authors: ["著者名"]
  language: ja
  profile: novel
  writing_mode: vertical-rl
  reading_direction: rtl

layout:
  binding: right
  chapter_start_page: odd
  allow_blank_pages: true

manuscript:
  chapters:
    - manuscript/01-prologue.md
    - manuscript/02-chapter-1.md

outputs:
  kindle:
    enabled: true
    target: kindle-ja

validation:
  strict: true
  epubcheck: true
  accessibility: warn

git:
  lfs: true
```

## 18. manga の最小例

```yaml
project:
  type: manga
  vcs: git

book:
  title: "サンプル漫画"
  authors: ["著者名"]
  language: ja
  profile: manga
  writing_mode: vertical-rl
  reading_direction: rtl

layout:
  binding: right
  chapter_start_page: any
  allow_blank_pages: true

outputs:
  kindle:
    enabled: true
    target: kindle-comic
  print:
    enabled: true
    target: print-manga

print:
  trim_size: B6
  bleed: 3mm
  crop_marks: true
  pdf_standard: pdfx1a

manga:
  reading_direction: rtl
  default_page_side: right
  spread_policy_for_kindle: split
  front_color_pages: 4
  body_mode: monochrome

validation:
  strict: true
  missing_image: error

git:
  lfs: true
```

## 19. schema バリデーションルール

- unknown key は v0.1 では warning
- enum 不一致は error
- `project.type: manga` なのに `manuscript.chapters` だけが存在する場合は warning
- `project.type != manga` なのに `manga` セクションが存在する場合は warning
- `outputs.print.enabled: true` なのに `print` セクションがない場合は warning
- `outputs.kindle.enabled: true` なのに `book.reading_direction` が未指定なら error

## 20. 将来拡張の余地

- JSON Schema 生成
- `custom_trim_size`
- section ごとの target override 詳細化
- printer preset
- cover schema の詳細定義
- page manifest schema の詳細定義
