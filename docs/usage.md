# Usage

`shosei` の現在の CLI surface、repo discovery の基本ルール、現時点で使える build 設定をまとめる。

この内容は `site/usage.html` と整合する前提で管理する。

## Typical flow

基本の流れは、初期化して、解決済み設定を確認し、build と validate を回すこと。

```bash
shosei init ./my-book --config-template novel
cd my-book
shosei explain
shosei build
shosei validate
```

## Commands

| Command | Purpose | Status |
|---|---|---|
| `shosei init` | project scaffold を作る | available |
| `shosei explain` | 解決済み設定と値の由来を表示する | available |
| `shosei build` | 有効な target の成果物を生成する | available |
| `shosei validate` | config / preflight を検証する | available |
| `shosei preview` | preview 導線 | placeholder |
| `shosei doctor` | 依存チェック導線 | placeholder |
| `shosei handoff <destination>` | handoff 導線 | placeholder |

## Repo discovery

`single-book` では root の `book.yml` を基準に動く。

`series` repo では、repo root から `--book <book-id>` を付けて実行するか、`books/<book-id>/...` の内側に移動して実行する。

```bash
shosei explain --book vol-01
shosei build --book vol-01
shosei validate --book vol-01
```

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

`init` はテンプレートに応じて、次のような土台を生成する。

- `book.yml` または `series.yml`
- `manuscript/` または `manga/`
- `assets/cover/`, `assets/images/`, `assets/fonts/`
- `styles/`
- `dist/`
- `.gitignore`, `.gitattributes`
