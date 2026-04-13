# Usage

`shosei` の現在の CLI surface、repo discovery の基本ルール、現時点で使える build 設定をまとめる。

この内容は `site/usage.html` と整合する前提で管理する。

## Typical flow

基本の流れは、初期化して、解決済み設定を確認し、build と validate を回すこと。

```bash
shosei init
shosei explain
shosei build
shosei validate
```

## Commands

| Command | Purpose | Status |
|---|---|---|
| `shosei init` | project scaffold を作る | 利用可能 |
| `shosei explain` | 解決済み設定と値の由来を表示する | 利用可能 |
| `shosei build` | 有効な target の成果物を生成する | 利用可能 |
| `shosei validate` | config / preflight を検証する | 利用可能 |
| `shosei preview` | one-shot / watch preview を生成する | 利用可能 |
| `shosei chapter <subcommand>` | prose の `manuscript.chapters` を更新する | 利用可能 |
| `shosei series sync` | series metadata と prose backmatter を同期する | 利用可能 |
| `shosei page check` | manga のページ順と見開き候補を検査する | 利用可能 |
| `shosei doctor` | 依存解決結果と導入ヒントを表示する | 利用可能 |
| `shosei handoff <destination>` | handoff package を生成する | 利用可能 |

## Repo discovery

`single-book` では root の `book.yml` を基準に動く。

`series` repo では、repo root から `--book <book-id>` を付けて実行するか、`books/<book-id>/...` の内側に移動して実行する。

```bash
shosei explain --book vol-01
shosei build --book vol-01
shosei validate --book vol-01
```

`build` / `validate` / `preview` では、`--target kindle|print` で対象 channel を絞れる。

```bash
shosei build --target print
shosei validate --target kindle
shosei preview --target print
```

`series sync` は `series.yml` を正として shared metadata を更新し、prose book では生成 backmatter を同期する。

```bash
shosei series sync
shosei series sync --path ./my-series
```

`page check` は manga project 向けで、ページ順や見開き候補を確認する。

```bash
shosei page check
shosei page check --book vol-01
```

## Chapter commands

`chapter` は prose project 向けで、`book.yml` の `manuscript.chapters` を更新する。

章順は filename prefix ではなく、この配列順で決まる。

```bash
shosei chapter add manuscript/03.md --title "Chapter 3"
shosei chapter move manuscript/03.md --before manuscript/02.md
shosei chapter remove manuscript/03.md
shosei chapter renumber
shosei chapter add books/vol-01/manuscript/02.md --book vol-01 --title "Chapter 2"
```

`page check` とは別系統で、`manga/pages/` や manga metadata には触れない。

`renumber` は章順を変えずに filename prefix だけを整える。`book.yml` の `manuscript.chapters` と対応する `sections.file` は更新するが、Markdown 本文中の link destination は自動 rewrite しない。

## Validate checks

現在の `validate` は、JSON レポートを出しつつ、次のような preflight を行う。

- build で必要になる `pandoc` の有無
- print build に設定された PDF engine の有無
- 欠落した manuscript / cover / manga page の検出
- prose 原稿のリンク切れと画像参照切れ
- prose 原稿の alt 欠落
- chapter ファイルの level-1 heading 不足
- heading hierarchy の飛び級
- Kindle / print / manga 向けの target 別警告

severity は `validation.accessibility`, `validation.missing_image`, `validation.missing_alt`, `validation.broken_link` の設定で調整できる。

## Inspect resolved config

`explain` は repo mode、対象 book、最終有効設定、値の由来を確認するための入口。

```bash
shosei explain
shosei explain --book vol-01
```

現在の `explain` では、たとえば次のような項目を確認できる。

- `cover.ebook_image`
- `outputs.kindle.target`
- `outputs.print.target`
- `pdf.engine`
- `pdf.toc`
- `pdf.page_number`
- `pdf.running_header`

## Print TOC

prose 系で print target を有効にした場合、`pdf.toc` で Pandoc に目次生成を指示できる。

```yaml
outputs:
  print:
    enabled: true
    target: print-jp-pdfx1a

pdf:
  engine: weasyprint
  toc: true
  page_number: true
  running_header: auto
```

既定では `pdf.toc: true`。

`pdf.toc: false` にすると、print build では Pandoc の `--toc` を付けずに実行する。

## Generated scaffold

`init` は標準では短い対話式で、作品カテゴリ、repo mode、タイトル、著者名、言語、出力先を確認してから scaffold を生成する。`--non-interactive --config-template <template>` を使うと既定値で生成できる。

テンプレートに応じて、次のような土台を生成する。

- `book.yml` または `series.yml`
- `manuscript/` または `manga/`
- `assets/cover/`, `assets/images/`, `assets/fonts/`
- `styles/`
- `dist/`
- `.gitignore`, `.gitattributes`
- `.agents/skills/shosei-project/SKILL.md`

prose 系テンプレートでは、最初の章ファイルとして `manuscript/01-chapter-1.md` も生成する。この `01-` prefix は初期命名の慣例で、章順の source of truth ではない。

## Preview and doctor

`preview` は one-shot と `--watch` をサポートする。`--watch` では `book.yml` / `series.yml`、原稿、styles、assets、`shared/` の変更を監視し、再生成失敗時も監視を継続する。

```bash
shosei preview --watch
shosei preview --watch --target print
```

`page check` は `dist/reports/<book-id>-page-check.json` を出しつつ、次を確認する。

- `manga/pages/` の辞書順ページ順
- 数値順と辞書順がずれるファイル名
- ページサイズの不一致
- 見開き候補と `manga.spread_policy_for_kindle` の整合
- `manga.front_color_pages` と `manga.body_mode` の整合

`doctor` は利用可能 / 不足 / pending を分けて表示し、次の依存について PATH 解決結果、バージョン、導入ヒントを返す。

- `pandoc`
- `epubcheck`
- `git`
- `git-lfs`
- `weasyprint`
- `typst`
- `lualatex`
- 代表 PDF engine
- Kindle Previewer
