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

series として始める場合は、次のように `--repo-mode series` を付ける。

```bash
shosei init ./my-series --non-interactive --config-template business --repo-mode series --title "My Series" --author "Ken" --language ja --output-preset both
cd my-series
shosei explain --book vol-01
shosei build --book vol-01
shosei validate --book vol-01
```

論文や発表前刷りは `paper` を使い、前刷り preset は `--config-profile conference-preprint` で選ぶ。

```bash
shosei init ./preprint --config-template paper --config-profile conference-preprint --non-interactive
cd preprint
shosei explain
shosei validate --target print
shosei build --target print
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
| `shosei story <subcommand>` | story workspace と scene map を扱う | 利用可能 |
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

`story scaffold` は物語補助の workspace を生成する。`single-book` では `story/`、`series` では共有 canon 用の `shared/metadata/story/` か巻固有の `books/<book-id>/story/` を作る。

```bash
shosei story scaffold
shosei story scaffold --book vol-01
shosei story scaffold --shared
shosei story map
shosei story map --book vol-01
shosei story check
shosei story check --book vol-01
shosei story drift --book vol-01
shosei story sync --book vol-01 --from shared --kind character --id lead
shosei story sync --book vol-01 --to shared --kind character --id lead
shosei story sync --book vol-01 --from shared --report dist/reports/vol-01-story-drift.json --force
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

## Story scaffold

`story scaffold` は manual-first の物語補助 workspace を作る。

- `single-book`: `story/`
- `series --shared`: `shared/metadata/story/`
- `series --book <book-id>`: `books/<book-id>/story/`

生成するもの:

- `README.md`
- `characters/README.md`
- `locations/README.md`
- `terms/README.md`
- `factions/README.md`
- book scope のときだけ `scenes.yml`

既定では既存 file を保持し、template を上書きしたい場合だけ `--force` を付ける。

## Story map

`story map` は book-scoped な `scenes.yml` を読み、scene 一覧と JSON report を出す。

- `single-book`: `story/scenes.yml`
- `series`: `books/<book-id>/story/scenes.yml`
- report: `single-book` は `dist/reports/default-story-map.json`、`series` は `dist/reports/<book-id>-story-map.json`

scene entry の最小 shape:

```yaml
scenes:
  - file: manuscript/01-chapter-1.md
    title: Opening
```

## Story check

`story check` は `scenes.yml`、scene Markdown frontmatter、book-scoped story entity Markdown を読み、軽い整合チェック結果を JSON report に出す。

- duplicate `file` entry は warning
- invalid repo-relative path は error
- 実ファイルが存在しない `file` は warning
- entity frontmatter の `id` は参照解決に使われ、未指定時は filename stem を使う
- 同一 kind 内の duplicate entity `id` は error
- `series` では book-scoped story data と `shared/metadata/story/` の両方から参照を解決する
- scene frontmatter の未解決 entity 参照は warning
- invalid scene/entity frontmatter は error
- report: `single-book` は `dist/reports/default-story-check.json`、`series` は `dist/reports/<book-id>-story-check.json`

## Story drift

`story drift` は `series` で shared canon と巻固有 story data の衝突を JSON report に出す。

- 対象: `shared/metadata/story/` と `books/<book-id>/story/`
- report には `drifts` 配列を含める
- same-scope duplicate entity `id` は error
- 内容が分岐した shared/book の同一 `id` は error
- 内容が同じ shared/book の同一 `id` は warning
- report: `dist/reports/<book-id>-story-drift.json`

## Story sync

`story sync` は `series` で shared canon と巻固有 story workspace の間を明示コピーする。1 entity の単体 sync と、`story drift` report からの batch sync を持つ。

- 例: `shosei story sync --book vol-01 --from shared --kind character --id lead`
- 例: `shosei story sync --book vol-01 --to shared --kind character --id lead`
- 例: `shosei story sync --book vol-01 --from shared --report dist/reports/vol-01-story-drift.json --force`
- `--from shared` か `--to shared` のどちらか一方を使う
- 単体 mode では `kind`: `character|location|term|faction`
- report mode では `--report` を使い、`--kind` / `--id` は使わない
- report mode は `--force` 必須
- destination 側に同じ `id` があり内容が違う場合は error
- `--force` を付けた場合だけ source 内容で destination 側を上書きする
- destination 側に同じ内容があれば no-op

## Validate checks

現在の `validate` は、JSON レポートを出しつつ、次のような preflight を行う。

- build で必要になる `pandoc` の有無
- print build に設定された PDF engine の有無
- 欠落した manuscript / cover / manga page の検出
- prose 原稿のリンク切れと画像参照切れ
- prose 原稿の alt 欠落
- chapter ファイルの level-1 heading 不足
- heading hierarchy の飛び級
- prose project の editorial sidecar に基づく表記ゆれ、claim / figure / freshness の検査
- Kindle / print / manga 向けの target 別警告
- `conference-preprint` profile の A4 / 2 段 / print preset 逸脱 warning

severity は `validation.accessibility`, `validation.missing_image`, `validation.missing_alt`, `validation.broken_link` の設定で調整できる。

issue の `location` は、特定できる場合は file path に加えて line 番号も持つ。
CLI では summary の後に、先頭最大 5 件の issue を `原因 / 発生箇所 / 修正例` の形で続けて表示する。

## Inspect resolved config

`explain` は repo mode、対象 book、最終有効設定、値の由来を確認するための入口。

```bash
shosei explain
shosei explain --book vol-01
shosei explain --json
```

現在の `explain` では、たとえば次のような項目を確認できる。

- `cover.ebook_image`
- `outputs.kindle.target`
- `outputs.print.target`
- `editorial.style`
- `editorial.claims`
- `editorial.figures`
- `editorial.freshness`
- `pdf.engine`
- `pdf.toc`
- `pdf.page_number`
- `pdf.running_header`
- `pdf.column_count`
- `pdf.column_gap`
- `pdf.base_font_size`
- `pdf.line_height`
- `print.trim_size`
- `print.page_margin`
- `print.sides`
- `print.max_pages`
- `print.pdf_standard`

editorial sidecar が設定されている場合、`explain` は rule / claim / figure / freshness item の件数も summary に出す。

editor integration 向けには `--json` も使える。resolved config の要約、主要 field の origin、chapter list などを機械可読で返す。

## Editorial sidecars

prose project では、`book.yml` から editorial 用 sidecar file を参照できる。

```yaml
editorial:
  style: editorial/style.yml
  claims: editorial/claims.yml
  figures: editorial/figures.yml
  freshness: editorial/freshness.yml
```

現在の `validate` は次を検査する。

- `style.yml` の推奨表記と禁止語
- `claims.yml` の source と section の整合
- `figures.yml` の asset / source と manuscript 参照の整合
- `freshness.yml` の参照整合と期限切れ

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

既定では `pdf.toc: true`。ただし `paper` / `conference-preprint` の scaffold では `pdf.toc: false` を書く。

`pdf.toc: false` にすると、print build では Pandoc の `--toc` を付けずに実行する。

`weasyprint` を使う print build では `styles/base.css`, `styles/print.css` と、`pdf` / `print` 設定から生成した layout stylesheet を合わせて渡す。`conference-preprint` では A4、余白、2 段組、本文サイズがこの generated stylesheet に反映される。`typst` では `columns`, `papersize`, `margin`, `fontsize`, `linestretch` の変数として渡す。

## Generated scaffold

`init` は標準では短い対話式で、作品カテゴリ、repo mode、タイトル、著者名、言語、出力先を確認してから scaffold を生成する。`--non-interactive --config-template <template>` を使うと既定値で生成できる。`--title`, `--author`, `--language`, `--output-preset`, `--repo-mode` で対話項目を explicit に上書きできる。`paper` では追加で `--config-profile paper|conference-preprint` を受け付ける。

テンプレートに応じて、次のような土台を生成する。

- `book.yml` または `series.yml`
- `dist/`
- `.gitignore`, `.gitattributes`
- `.agents/skills/shosei-project/SKILL.md`
- `single-book` では `assets/cover/`, `assets/images/`, `assets/fonts/`, `styles/`
- `series` では `shared/assets/`, `shared/styles/`, `shared/fonts/`, `shared/metadata/`, `books/<book-id>/assets/`
- prose 系では `single-book` に原稿ファイルと `editorial/*.yml`、`series` に `books/<book-id>/manuscript/` と `books/<book-id>/editorial/*.yml`
- manga 系では `single-book` に `manga/`、`series` に `books/<book-id>/manga/`

prose 系テンプレートでは、最初の原稿ファイルとして `paper` / `conference-preprint` は `single-book` で `manuscript/01-main.md`、`series` で `books/<book-id>/manuscript/01-main.md` を生成する。その他の prose は `01-chapter-1.md` を生成する。この `01-` prefix は初期命名の慣例で、章順の source of truth ではない。

また、prose 系では空の `editorial/style.yml`, `editorial/claims.yml`, `editorial/figures.yml`, `editorial/freshness.yml` を生成し、`single-book` では `book.yml`、`series` では `books/<book-id>/book.yml` から参照する。style 側は `single-book` では `styles/base.css`, `styles/epub.css`, `styles/print.css`、`series` では `shared/styles/base.css` を生成する。

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

summary には page order と spread candidates も出る。

CLI では summary の後に、先頭最大 5 件の issue を `原因 / 発生箇所 / 修正例` の形で続けて表示する。

`doctor` は required / optional を分けて表示し、PATH 解決結果、バージョン、導入ヒントを返す。

```bash
shosei doctor
shosei doctor --json
```

- `git`
- `pandoc`
- `weasyprint`
- `epubcheck`
- `git-lfs`
- Kindle Previewer

required tool は `git`, `pandoc`, `weasyprint`。optional tool は `epubcheck`, `git-lfs`, Kindle Previewer。

`typst`, `lualatex` は将来拡張候補として config 値では受け付けるが、v0.1 の doctor の必須確認対象には含めない。

`--json` は editor integration 向けで、host OS、required / optional ごとの available / missing / pending 件数、各 tool の category / status / path / version / install hint を機械可読で返す。

`handoff proof` は validate report に加えて、`review-notes.md`、`reports/review-packet.json`、editorial sidecar のコピーも package に含める。`manifest.json` には review packet の path と editorial summary 件数も入る。
