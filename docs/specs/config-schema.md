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
cover:
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
| `cover` | no | all | 外部カバー画像 |
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
| `type` | string | yes | none | `business`, `paper`, `novel`, `light-novel`, `manga` |
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
| `profile` | string | yes | derived from `project.type` | `business`, `paper`, `conference-preprint`, `novel`, `light-novel`, `manga` |
| `writing_mode` | string | yes | profile-based | `horizontal-ltr`, `vertical-rl` |
| `reading_direction` | string | yes | derived from `writing_mode` | `ltr`, `rtl` |
| `identifier` | string | no | `auto` | any or `auto` |
| `rights` | string | no | empty string | any |
| `audience` | string | no | `general` | `general`, `children`, `ya`, `professional` |

制約:

- `profile: manga` は `project.type: manga` のときのみ許可
- `profile: conference-preprint` は `project.type: paper` のときのみ許可
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

## 7. `cover`

外部カバーアセットを定義する。本文フローの `manuscript` / `sections` とは別に扱う。

```yaml
cover:
  ebook_image: assets/cover/front.jpg
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `ebook_image` | string | no | none | repo-relative image path |

制約:

- `ebook_image` は repo root 基準の相対パス
- v0.1 では `.jpg`, `.jpeg`, `.png` を許可
- `cover.ebook_image` は Kindle/EPUB 向けの外部カバー画像を指す
- `manuscript` や `sections.type: cover` の本文ページ指定とは別概念

## 8. `manuscript`

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
- prose の章順は `manuscript.chapters` の配列順で決まる
- filename prefix は順序の source of truth ではない
- `01-`, `02-` などの prefix は scaffold と互換な命名慣例に留める

## 9. `sections`

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
- `type: cover` は本文フロー内のカバーページを表し、`cover.ebook_image` の代替にはならない
- `sections` はファイルの意味属性のみを持ち、章題や節題の文字列は保持しない

### 9.1 見出しと navigation

v0.1 では `book.yml` は見出し文字列の primary source にはしない。

既定:

- `project.type != manga` の場合、navigation は Markdown 見出しから導出する
- `manuscript.chapters` の各本文ファイルでは、最初の level-1 heading を chapter title として扱う
- chapter title の取得元と章順の取得元は分け、章順そのものは `manuscript.chapters` を参照する
- `project.type: manga` の場合、navigation heading は必須ではなく、page order が primary source となる
- TOC / EPUB nav / PDF bookmark / running header は導出済み navigation structure を参照する
- v0.1 では `sections` に `title` や `short_title` の override field を持たせない

## 10. `outputs`

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

## 11. `pdf`

`outputs.print.enabled: true` の場合に推奨。`project.type != manga` で PDF backend を指定する。

```yaml
pdf:
  engine: chromium
  toc: true
  page_number: true
  running_header: auto
  column_count: 1
  column_gap: auto
  base_font_size: auto
  line_height: auto
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `engine` | string | conditional | `writing_mode` / `profile` derived | `weasyprint`, `chromium`, `typst`, `lualatex` |
| `toc` | boolean | no | `true` | `true`, `false` |
| `page_number` | boolean | no | `true` | `true`, `false` |
| `running_header` | string | no | `auto` | `auto`, `none`, `title`, `chapter` |
| `column_count` | integer | no | `1` | positive integer |
| `column_gap` | string | no | `auto` | CSS length string or `auto` |
| `base_font_size` | string | no | `auto` | CSS length string or `auto` |
| `line_height` | string | no | `auto` | CSS length string or `auto` |

補足:

- `book.writing_mode: vertical-rl` の prose print 既定 engine は `chromium`
- `book.writing_mode: horizontal-ltr` の prose print 既定 engine は `weasyprint`
- `book.profile: conference-preprint` は `weasyprint` を正式既定とする
- `weasyprint` は現行 v0.1 経路では `vertical-rl` prose print を表現できないため、`book.writing_mode: vertical-rl` と組み合わせる場合は error とする
- `typst`, `lualatex` は将来拡張・検証候補として値は受け付けるが、v0.1 の doctor / CI の必須サポート対象には含めない
- `toc: true` は prose の導出済み navigation structure から目次を生成する
- `book.writing_mode: vertical-rl` の generated page style は Chromium の margin box 挙動に合わせて中央寄せを既定にする
  - `page_number: true` の場合は page number を各ページの下中央に置く
  - `running_header != none` の場合は running header を各ページの上中央に置く
  - title / TOC など frontmatter では page number と running header を抑制する
- `running_header: chapter` は prose 本文の chapter title を参照する
- `running_header: auto` は profile ごとの既定を使い、必要に応じて chapter title を参照する
- `book.profile: conference-preprint` では `toc: false`, `page_number: false`, `running_header: none`, `column_count: 2`, `column_gap: 10mm`, `base_font_size: 9pt`, `line_height: 14pt` を既定候補とする

## 12. `print`

```yaml
print:
  trim_size: A4
  bleed: 0mm
  crop_marks: false
  page_margin:
    top: 20mm
    bottom: 20mm
    left: 15mm
    right: 15mm
  sides: duplex
  max_pages: 2
  body_pdf: true
  cover_pdf: false
  pdf_standard: pdfx1a
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `trim_size` | string | conditional | `A5` | `A4`, `A5`, `B6`, `bunko`, `custom` |
| `bleed` | string | no | `3mm` | CSS length string |
| `crop_marks` | boolean | no | `true` | `true`, `false` |
| `page_margin` | object | no | profile-based | `top`, `bottom`, `left`, `right` を持つ margin object |
| `sides` | string | no | `simplex` | `simplex`, `duplex` |
| `max_pages` | integer | no | none | positive integer |
| `body_pdf` | boolean | no | `true` | `true`, `false` |
| `cover_pdf` | boolean | no | `false` | `true`, `false` |
| `pdf_standard` | string | no | derived from target | `pdfx1a`, `pdfx4` |
| `body_mode` | string | no | `auto` | `auto`, `monochrome`, `color` |

制約:

- `trim_size: custom` の場合は将来 `custom_trim_size` の追加が必要
- `cover_pdf: true` は v0.1 では metadata だけ先に定義し、実装は将来に回してもよい
- `page_margin` を指定する場合は `top`, `bottom`, `left`, `right` の 4 辺を揃えて指定する
- `book.profile: conference-preprint` では `trim_size: A4`, `bleed: 0mm`, `crop_marks: false`, `page_margin.top/bottom: 20mm`, `page_margin.left/right: 15mm`, `sides: duplex`, `max_pages: 2` を既定候補とする

## 13. `images`

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

## 14. `validation`

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

## 15. `git`

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

## 16. `editorial`

prose 系 project で任意。editorial metadata の sidecar file を参照する。

```yaml
editorial:
  style: editorial/style.yml
  claims: editorial/claims.yml
  figures: editorial/figures.yml
  freshness: editorial/freshness.yml
```

| Field | Type | Required | Default | Allowed |
|---|---|---|---|---|
| `style` | string | no | none | repo-relative YAML path |
| `claims` | string | no | none | repo-relative YAML path |
| `figures` | string | no | none | repo-relative YAML path |
| `freshness` | string | no | none | repo-relative YAML path |

制約:

- 参照先は repo root 基準の相対 path
- prose 系では `validate`, `explain`, `handoff proof` が参照できる
- v0.1 では sidecar 自体の merge は行わない

### 16.1 `style.yml`

```yaml
preferred_terms:
  - preferred: "Git"
    aliases:
      - "git"
    severity: warn
banned_terms:
  - term: "出来る"
    severity: warn
    reason: 常用表記に寄せる
```

- `preferred_terms[].preferred` は正規表記
- `preferred_terms[].aliases` は検出対象
- `banned_terms[].term` は禁止語
- `severity` は `warn | error`

### 16.2 `claims.yml`

```yaml
claims:
  - id: claim-market-size
    summary: 国内市場は拡大している
    section: manuscript/02-market.md
    sources:
      - https://example.com/report
      - "ref:market-report-2026"
    reviewer_note: 数値の更新を release 前に確認
```

- `id` は file 内で一意
- `section` は prose manuscript 内の file path
- `sources` は 1 件以上を推奨する
- `sources` の `ref:<id>` は reference entry id を表す明示記法
- `single-book` では `references/entries/`、`series` の巻固有 scope では book 側と shared 側の reference id に解決できる

### 16.3 `figures.yml`

```yaml
figures:
  - id: fig-architecture
    path: assets/images/architecture.png
    caption: 全体構成
    source: 社内図を再構成
    rights: owned
    reviewer_note: ロゴ差し替え待ち
```

- `id` は file 内で一意
- `path` は repo-relative asset path
- `source` と `rights` は校正・入稿時の確認対象

### 16.4 `freshness.yml`

```yaml
tracked:
  - kind: claim
    id: claim-market-size
    last_verified: 2026-04-13
    review_due_on: 2026-05-13
    note: 市場規模の数字は月次で見直す
```

- `kind` は `claim | figure`
- `id` は対応する claim / figure id を参照する
- 日付は `YYYY-MM-DD`
- `review_due_on` を過ぎた項目は warning 以上で報告する

## 17. `manga`

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

補足:

- v0.1 では chapter title や section title の metadata は必須ではない
- Kindle 向け nav が必要な場合でも、既定では page sequence を使って生成する

## 18. `pipeline`

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

## 19. prose の最小例

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

cover:
  ebook_image: assets/cover/front.jpg

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

editorial:
  style: editorial/style.yml
  claims: editorial/claims.yml
  figures: editorial/figures.yml
  freshness: editorial/freshness.yml
```

## 20. paper の最小例

```yaml
project:
  type: paper
  vcs: git

book:
  title: "サンプル論文"
  authors: ["著者名"]
  language: ja
  profile: paper
  writing_mode: horizontal-ltr
  reading_direction: ltr

layout:
  binding: left
  chapter_start_page: any
  allow_blank_pages: false

manuscript:
  chapters:
    - manuscript/01-main.md

outputs:
  print:
    enabled: true
    target: print-jp-pdfx4

pdf:
  engine: weasyprint
  toc: false
  page_number: true
  running_header: none

validation:
  strict: true

git:
  lfs: true
```

## 21. conference-preprint の最小例

```yaml
project:
  type: paper
  vcs: git

book:
  title: "サンプル前刷り"
  authors: ["著者名"]
  language: ja
  profile: conference-preprint
  writing_mode: horizontal-ltr
  reading_direction: ltr

layout:
  binding: left
  chapter_start_page: any
  allow_blank_pages: false

manuscript:
  chapters:
    - manuscript/01-main.md

outputs:
  print:
    enabled: true
    target: print-jp-pdfx4

pdf:
  engine: weasyprint
  toc: false
  page_number: false
  running_header: none
  column_count: 2
  column_gap: 10mm
  base_font_size: 9pt
  line_height: 14pt

print:
  trim_size: A4
  bleed: 0mm
  crop_marks: false
  page_margin:
    top: 20mm
    bottom: 20mm
    left: 15mm
    right: 15mm
  sides: duplex
  max_pages: 2
  body_pdf: true
  cover_pdf: false
  pdf_standard: pdfx4

validation:
  strict: true

git:
  lfs: true
```

## 22. manga の最小例

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

## 23. schema バリデーションルール

- unknown key は v0.1 では warning
- enum 不一致は error
- `cover.ebook_image` が存在する場合、拡張子不正は error
- `project.type: manga` なのに `manuscript.chapters` だけが存在する場合は warning
- `project.type != manga` なのに `manga` セクションが存在する場合は warning
- `outputs.print.enabled: true` なのに `print` セクションがない場合は warning
- `outputs.kindle.enabled: true` なのに `book.reading_direction` が未指定なら error
- `book.writing_mode: vertical-rl` なのに `pdf.engine = weasyprint` の場合は error
- `editorial.*` がある場合、参照先 path の形式不正は error
- `book.profile: conference-preprint` なのに `outputs.print.enabled` が `true` でない場合は warning
- `book.profile: conference-preprint` なのに `pdf.engine != weasyprint` の場合は warning
- `book.profile: conference-preprint` なのに `pdf.toc != false` の場合は warning
- `book.profile: conference-preprint` なのに `pdf.page_number != false` の場合は warning
- `book.profile: conference-preprint` なのに `pdf.running_header != none` の場合は warning
- `book.profile: conference-preprint` なのに `pdf.column_count != 2` の場合は warning
- `book.profile: conference-preprint` なのに `pdf.column_gap != 10mm` の場合は warning
- `book.profile: conference-preprint` なのに `pdf.base_font_size != 9pt` の場合は warning
- `book.profile: conference-preprint` なのに `pdf.line_height != 14pt` の場合は warning
- `book.profile: conference-preprint` なのに `print.trim_size != A4` の場合は warning
- `book.profile: conference-preprint` なのに `print.bleed != 0mm` の場合は warning
- `book.profile: conference-preprint` なのに `print.crop_marks != false` の場合は warning
- `book.profile: conference-preprint` なのに `print.page_margin.top/bottom != 20mm` または `left/right != 15mm` の場合は warning
- `book.profile: conference-preprint` なのに `print.sides != duplex` の場合は warning
- `book.profile: conference-preprint` なのに `print.max_pages != 2` の場合は warning
- `manga.front_color_pages` が resolved page count を超える場合は error
- `manga.body_mode: monochrome` で `front_color_pages` を超えた本文ページに color 画像がある場合は error

## 24. 将来拡張の余地

- JSON Schema 生成
- `custom_trim_size`
- section ごとの target override 詳細化
- printer preset
- print cover source schema の詳細定義
- page manifest schema の詳細定義
