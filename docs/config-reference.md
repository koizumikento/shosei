# Config Reference

`shosei init` が生成する `book.yml` / `series.yml` / `books/<book-id>/book.yml` の意味を、普段触る項目に絞ってまとめる。

正式な schema と制約は次を参照する。

- [設定ファイル schema](specs/config-schema.md)
- [series.yml schema](specs/series-schema.md)
- [設定探索と継承ルール](specs/config-loading.md)

## 1. どのファイルを見るか

### `single-book`

- repo root の `book.yml` を編集する

### `series`

- シリーズ共通項目は root の `series.yml`
- 巻固有の項目は `books/<book-id>/book.yml`

基本方針:

- シリーズ名、shared path、共通 defaults は `series.yml`
- その巻のタイトル、著者、原稿 path、editorial path は `books/<book-id>/book.yml`

## 2. `single-book` の基本形

```yaml
project:
  type: novel
  vcs: git
  version: 1
book:
  title: "My Book"
  authors:
    - "Ken"
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
    - manuscript/01-chapter-1.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
images:
  epub_figure_layout: auto
validation:
  strict: true
  epubcheck: true
  kindle_previewer: false
  accessibility: warn
git:
  lfs: true
editorial:
  style: editorial/style.yml
  claims: editorial/claims.yml
  figures: editorial/figures.yml
  freshness: editorial/freshness.yml
```

## 3. `series` の役割分担

### `series.yml`

```yaml
series:
  id: my-series
  title: "My Series"
  language: ja
  type: novel
shared:
  assets:
    - shared/assets
  styles:
    - shared/styles
  fonts:
    - shared/fonts
  metadata:
    - shared/metadata
defaults:
  book:
    profile: novel
    writing_mode: vertical-rl
    reading_direction: rtl
  layout:
    binding: right
    chapter_start_page: odd
    allow_blank_pages: true
  images:
    epub_figure_layout: auto
books:
  - id: vol-01
    path: books/vol-01
    number: 1
    title: "Volume 1"
```

### `books/<book-id>/book.yml`

```yaml
project:
  type: novel
  vcs: git
  version: 1
book:
  title: "Volume 1"
  authors:
    - "Ken"
  language: ja
layout:
  binding: right
  chapter_start_page: odd
  allow_blank_pages: true
manuscript:
  chapters:
    - books/vol-01/manuscript/01-chapter-1.md
editorial:
  style: books/vol-01/editorial/style.yml
  claims: books/vol-01/editorial/claims.yml
  figures: books/vol-01/editorial/figures.yml
  freshness: books/vol-01/editorial/freshness.yml
```

## 4. よく触る項目

| Field | どこにあるか | 意味 |
|---|---|---|
| `project.type` | `book.yml` | プロジェクト種別。`business`, `paper`, `novel`, `light-novel`, `manga` |
| `book.title` | `book.yml`, `books/<book-id>/book.yml` | その本のタイトル |
| `book.authors` | `book.yml`, `books/<book-id>/book.yml` | 著者一覧 |
| `book.language` | どちらにもありうる | 本文の言語コード |
| `book.profile` | 主に `book.yml` か `series.yml.defaults.book` | 出力 preset。`conference-preprint` などの細分化を含む |
| `book.writing_mode` | 主に `book.yml` か `series.yml.defaults.book` | `horizontal-ltr` か `vertical-rl` |
| `book.reading_direction` | 主に `book.yml` か `series.yml.defaults.book` | `ltr` か `rtl` |
| `layout.binding` | `book.yml` または `series.yml.defaults.layout` | 綴じ方向。横組みは通常 `left`、縦組みは通常 `right` |
| `layout.chapter_start_page` | 同上 | 章を奇数ページ始まりにするか。`odd` か `any` |
| `layout.allow_blank_pages` | 同上 | レイアウト調整のために空ページを入れてよいか |
| `manuscript.chapters` | prose の `book.yml` | 章順そのもの。filename prefix よりこちらが優先 |
| `outputs.kindle.enabled` | `book.yml` または `series.yml.defaults.outputs` | Kindle 出力を有効化するか |
| `outputs.kindle.target` | 同上 | prose は通常 `kindle-ja`、manga は `kindle-comic` |
| `outputs.print.enabled` | 同上 | print 出力を有効化するか |
| `outputs.print.target` | 同上 | `print-jp-pdfx1a`, `print-jp-pdfx4`, `print-manga` |
| `images.epub_figure_layout` | `book.yml` または `series.yml.defaults.images` | EPUB の図版レイアウト。`auto` は light-novel だけ standalone、それ以外は inline |
| `pdf.engine` | prose の print 時 | print PDF のレンダラ。`chromium` や `weasyprint` |
| `print.trim_size` | print 時 | 仕上がりサイズ。`A4`, `bunko` など |
| `editorial.style` | prose の `book.yml` | 用語や表記ルールの sidecar file |
| `editorial.claims` | prose の `book.yml` | 要出典・要確認の claim 台帳 |
| `editorial.figures` | prose の `book.yml` | 図版台帳 |
| `editorial.freshness` | prose の `book.yml` | 更新期限を持つ facts の台帳 |
| `validation.strict` | `book.yml` または `series.yml` | 厳しめの検証を有効にするか |
| `validation.epubcheck` | `book.yml` または `series.yml` | Kindle/EPUB 出力で `epubcheck` を使うか |
| `validation.kindle_previewer` | `book.yml` または `series.yml` | Kindle Previewer の device-oriented conversion check を opt-in するか |
| `git.lfs` | `book.yml` または `series.yml` | Git LFS を前提にするか |

## 5. `series.yml` でよく触る項目

| Field | 意味 |
|---|---|
| `series.id` | シリーズ識別子。生成 metadata に使う |
| `series.title` | シリーズ名 |
| `shared.assets` | shared cover や画像を置く探索先 |
| `shared.styles` | shared CSS を置く探索先 |
| `shared.fonts` | shared font を置く探索先 |
| `shared.metadata` | series catalog など shared metadata の置き場 |
| `defaults.book.*` | 各巻の既定値 |
| `defaults.layout.*` | 各巻の layout 既定値 |
| `defaults.images.*` | 各巻の画像レイアウト既定値 |
| `books[].id` | `--book` で指定する巻 ID |
| `books[].path` | 巻ディレクトリの repo-relative path |
| `books[].number` | 巻番号 |
| `books[].title` | 巻タイトル |

## 6. `manga` の項目

`project.type: manga` のときは prose の `manuscript` / `editorial` ではなく `manga` block を使う。

| Field | 意味 |
|---|---|
| `manga.reading_direction` | ページ読み順 |
| `manga.default_page_side` | 本文の最初のページを左右どちらに置くか |
| `manga.spread_policy_for_kindle` | 見開きを Kindle 向けにどう落とすか |
| `manga.front_color_pages` | 巻頭カラーの枚数 |
| `manga.body_mode` | 本文ページの既定。通常は `monochrome` |

## 7. 触り方の目安

日常的に触るのは主に次。

- タイトルや著者を変える
- `manuscript.chapters` の順番を確認する
- `outputs` の有効 / 無効を切り替える
- print 用の `pdf` / `print` を詰める
- prose なら `editorial/*.yml` を埋める

あまり触らないもの:

- `project.version`
- `git.lfs`
- `shared.*` の path

## 8. 注意点

- config の path は repo-relative
- path 区切りは `/`
- prose の章順は `manuscript.chapters` の配列順で決まる
- EPUB で図とキャプションを同じページに保持したい場合は、Markdown 画像の `[]` に表示キャプションを入れる。画像直後の `*図：...*` 段落は `figure` 外に出るため、`images.epub_figure_layout: standalone` とは相性が悪い
- `series` では `book.yml` より `series.yml` の defaults が先に入り、巻固有 `book.yml` が上書きする
- 詳しい制約や許容値は schema を参照する
