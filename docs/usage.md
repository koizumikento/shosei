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

この表の `利用可能` は、CLI surface が実装済みで、CLI smoke または workspace test で継続確認している意味で使う。外部 validator は次のように扱う。

| Validator depth | Status |
|---|---|
| local lint / target-profile checks | 標準で利用可能 |
| `epubcheck` / `qpdf` | tool があれば実行し、なければ `missing-tool` として report に記録 |
| Kindle Previewer | `validation.kindle_previewer: true` の opt-in。CI は fake executable で report contract を確認し、実物確認は maintainers の local hook で行う |
| Kindle Previewer 以外の store / device 固有 validator | future work |

| Command | Purpose | Status |
|---|---|---|
| `shosei init` | project scaffold を作る | 利用可能 |
| `shosei explain` | 解決済み設定と値の由来を表示する | 利用可能 |
| `shosei build` | 有効な target の成果物を生成する | 利用可能 |
| `shosei validate` | config / preflight を検証する | 利用可能 |
| `shosei preview` | one-shot / watch preview を生成する | 利用可能 |
| `shosei chapter <subcommand>` | prose の `manuscript.chapters` を更新する | 利用可能 |
| `shosei reference <subcommand>` | 参考資料 workspace と entry 一覧 / check / drift / sync を扱う | 利用可能 |
| `shosei story <subcommand>` | story workspace と scene map を扱う | 利用可能 |
| `shosei series sync` | series metadata と prose backmatter を同期する | 利用可能 |
| `shosei page check` | manga のページ順と見開き候補を検査する | 利用可能 |
| `shosei doctor` | 依存解決結果と導入ヒントを表示する | 利用可能 |
| `shosei handoff <kindle|print|proof>` | handoff package を生成する | 利用可能 |

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

`validate` では target / profile の組み合わせに対する実務上の warning も出す。

- `project.type: manga` では `outputs.kindle.target: kindle-comic` を推奨し、`kindle-ja` は将来互換扱いとして warning にする
- `project.type: manga` では `outputs.print.target: print-manga` を推奨し、prose 向け print target は warning にする
- `outputs.print.target: print-jp-pdfx1a` と `print.pdf_standard: pdfx4` のような target / PDF standard の不一致は warning にする
- `book.profile: conference-preprint` では `outputs.print.target: print-jp-pdfx4` を推奨し、それ以外は warning にする
- prose print で `pdf.engine: typst|lualatex` を使う場合は、v0.2 では追加 proof を勧める warning を出す

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

`reference scaffold` は参考リンクや作業メモの workspace を生成する。`single-book` では `references/`、`series` では共有用の `shared/metadata/references/` か巻固有の `books/<book-id>/references/` を作る。

```bash
shosei reference scaffold
shosei reference scaffold --book vol-01
shosei reference scaffold --shared
shosei reference map
shosei reference map --book vol-01
shosei reference map --shared
shosei reference check
shosei reference check --book vol-01
shosei reference check --shared
shosei reference drift --book vol-01
shosei reference sync --book vol-01 --from shared --id market
shosei reference sync --book vol-01 --to shared --id market
shosei reference sync --book vol-01 --from shared --report dist/reports/vol-01-reference-drift.json --force
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

## Reference scaffold

`reference scaffold` は manual-first の参考資料 workspace を作る。

- `single-book`: `references/`
- `series --shared`: `shared/metadata/references/`
- `series --book <book-id>`: `books/<book-id>/references/`

生成するもの:

- `README.md`
- `entries/README.md`

既定では既存 file を保持し、template を上書きしたい場合だけ `--force` を付ける。

## Reference map

`reference map` は reference workspace の `entries/` を読み、entry 一覧と JSON report を出す。

- `single-book`: `references/entries/`
- `series --shared`: `shared/metadata/references/entries/`
- `series --book <book-id>`: `books/<book-id>/references/entries/`
- report: `single-book` は `dist/reports/default-reference-map.json`、`series --shared` は `dist/reports/shared-reference-map.json`、`series --book <book-id>` は `dist/reports/<book-id>-reference-map.json`

entry frontmatter の最小 shape:

```yaml
id: market-report-2026
title: 2026年国内市場レポート
links:
  - https://example.com/report
tags:
  - market
status: unread
```

- `id` は frontmatter 優先、未指定時は filename stem
- `title`, `status` は任意
- `links`, `tags`, `related_sections` は任意の string 配列
- `README.md` は scan 対象外

## Reference check

`reference check` は reference workspace の `entries/` を読み、frontmatter shape、duplicate `id`、local path を軽く検査して JSON report を出す。prose book では、`editorial.claims.yml` の `sources` にある `ref:<id>` も照合する。

- `single-book`: `references/entries/`
- `series --shared`: `shared/metadata/references/entries/`
- `series --book <book-id>`: `books/<book-id>/references/entries/`
- report: `single-book` は `dist/reports/default-reference-check.json`、`series --shared` は `dist/reports/shared-reference-check.json`、`series --book <book-id>` は `dist/reports/<book-id>-reference-check.json`

v0.2 の検査対象:

- invalid / unclosed frontmatter
- `id` の空文字や duplicate `id`
- `links` の local repo path
- `related_sections` の repo-relative path
- `claims.yml` の `ref:<id>`
- local path が存在しない場合は warning
- 解決できない `ref:<id>` は error
- `claims.yml` の `ref:<id>` は `single-book` では current book、`series --book <book-id>` では current book と shared の reference id に解決する
- `https://`, `http://`, `mailto:`, `tel:`, `#anchor` は存在確認しない

## Reference drift

`reference drift` は `series` の shared reference と巻固有 reference の衝突と gap を JSON report に出す。

- 対象: `shared/metadata/references/entries/` と `books/<book-id>/references/entries/`
- report: `dist/reports/<book-id>-reference-drift.json`
- 同じ `id` が shared と book の両方にある entry だけを比較する
- 同じ内容なら `redundant-copy` として warning
- 異なる内容なら `drift` として error
- shared にだけある entry は `shared-only`
- book にだけある entry は `book-only`
- invalid frontmatter や same-scope duplicate `id` も issue に含める
- `entries/` directory が存在しない scope は empty として扱う

## Reference sync

`reference sync` は `series` で shared reference と巻固有 reference の間を明示コピーする。単体 sync と、`reference drift` report からの batch sync を持つ。

- 例: `shosei reference sync --book vol-01 --from shared --id market`
- 例: `shosei reference sync --book vol-01 --to shared --id market`
- 例: `shosei reference sync --book vol-01 --from shared --report dist/reports/vol-01-reference-drift.json --force`
- `--from shared` か `--to shared` のどちらか一方を使う
- 単体 mode では `id` を 1 件指定する
- report mode では `--report` を使い、`--id` は使わない
- report mode は `--force` 必須
- report mode では `drifts` に加えて source 側にある `gaps` も適用する
- `--from shared` は `shared-only` gap を適用し、`book-only` gap は skip する
- `--to shared` は `book-only` gap を適用し、`shared-only` gap は skip する
- destination 側に同じ `id` があり内容が違う場合は error
- `--force` を付けた場合だけ source 内容で destination 側を上書きする
- destination 側に同じ内容があれば no-op
- destination 側に同じ `id` が無い場合は source filename を引き継いで新規作成する

## Story scaffold

`story scaffold` は manual-first の物語補助 workspace を作る。

- `single-book`: `story/`
- `series --shared`: `shared/metadata/story/`
- `series --book <book-id>`: `books/<book-id>/story/`

生成するもの:

- `README.md`
- `characters/README.md`
- `characters/_template.md`
- `locations/README.md`
- `locations/_template.md`
- `terms/README.md`
- `terms/_template.md`
- `factions/README.md`
- `factions/_template.md`
- book scope のときだけ `scenes.yml`
- book scope のときだけ `scene-template.md`
- book scope のときだけ `structures/README.md`
- book scope のときだけ `structures/kishotenketsu.md`
- book scope のときだけ `structures/three-act.md`
- book scope のときだけ `structures/save-the-cat.md`
- book scope のときだけ `structures/heroes-journey.md`

既定では既存 file を保持し、template を上書きしたい場合だけ `--force` を付ける。

`_template.md` と `scene-template.md` は日本語中心の記入例。`structures/*.md` は book-scoped な構成メモの叩き台。entity dir の `_template.md` は `story check` / `story drift` / `story sync` の scan 対象に含めない。CLI が意味を持って読む key は `id`, `characters`, `locations`, `terms`, `factions`, `scenes`, `file`, `title` のように英語のまま使い、`structures/` 配下では `story seed` 用の `scene_seeds` frontmatter だけを読む。

## Story seed

`story seed` は book-scoped な `structures/<template>.md` の `scene_seeds` frontmatter を使って `scenes.yml` と `scene-notes/*.md` の下書きを起こす。

- `single-book`: `story/structures/<template>.md`
- `series`: `books/<book-id>/story/structures/<template>.md`
- `--template` は file stem か `.md` 付き file 名
- `scene_seeds[*].file` を省略した場合は `scene-notes/<nn>-scene.md` を自動採番
- 既存 scene note は既定で保持し、`--force` を付けたときだけ seed 内容で上書き
- 非空の `scenes.yml` を置き換える場合は `--force` が必要

`scene_seeds` の最小 shape:

```yaml
scene_seeds:
  - title: 起: 日常の提示
    beat: 起
    summary: 主人公の日常と物語の約束を見せる
    characters:
      - 主人公
```

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
- `README.md` と `_template.md` は entity scan 対象に含めない
- `series` では book-scoped story data と `shared/metadata/story/` の両方から参照を解決する
- scene frontmatter の `characters`, `locations`, `terms`, `factions` の未解決 entity 参照は warning
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

現在の `validate` は、`dist/reports/<book-id>-validate.json` に JSON レポートを書き出しつつ、次のような preflight を行う。

- build で必要になる `pandoc` の有無
- print build に設定された PDF engine の有無
- 欠落した manuscript / cover / manga page の検出
- prose 原稿のリンク切れと画像参照切れ
- prose 原稿の alt 欠落
- chapter ファイルの level-1 heading 不足
- heading hierarchy の飛び級
- prose 原稿の文字数集計
- prose project の editorial sidecar に基づく表記ゆれ、claim / figure / freshness の検査
- Kindle / print / manga 向けの target 別警告
- `conference-preprint` profile の print preset 逸脱 warning

severity は `validation.accessibility`, `validation.missing_image`, `validation.missing_alt`, `validation.broken_link` の設定で調整できる。

文字数は YAML frontmatter を除き、Markdown 記法を落とした plain text を基準に集計する。summary には total character count を出し、JSON report には `manuscript_stats` として total / frontmatter / chapters / backmatter と file ごとの内訳を含める。

issue の `location` は、特定できる場合は file path に加えて line 番号も持つ。
CLI では summary の後に、先頭最大 5 件の issue を `原因 / 発生箇所 / 修正例` の形で続けて表示する。

`validate --json` は同じ report schema を stdout に出す。file へ書き出す report には `checks`, `target_profile_validations`, `validators`, `issues` を含み、prose では `manuscript_stats` も含む。`target_profile_validations` には Kindle / print の target、book profile、target / PDF standard / profile default の判定 summary が入る。external validator を実行した場合は `artifact`, `log_path`, `summary` も取れる。

現在の `validate` は local lint と tool availability check に加えて、Kindle 出力が有効で `validation.epubcheck: true` のときは生成した EPUB に対して `epubcheck` を走らせる。Kindle 出力が有効で `validation.kindle_previewer: true` のときは、生成した Kindle artifact に対して Kindle Previewer の conversion check も試みる。print 出力が有効なときは生成した PDF に対して `qpdf --check` も試みる。`epubcheck`, `qpdf`, Kindle Previewer が未導入でも `validators[]` に `missing-tool` として記録し、validation 自体はそれだけでは fail にしない。これらの validator の検査失敗は validation error として返す。validator の詳細ログは `dist/logs/` に保存し、report から参照できる。validator 前提の build が失敗した場合も validator entry 自体は `failed` または `skipped` として残り、`summary` と `log_path` で build prerequisite failure を読める。

validator confidence は次の層で読む。

| Layer | What is proven | How to use it |
|---|---|---|
| Local structural checks | config / preflight / manuscript / target-profile checks | `shosei validate` の標準結果として読む |
| Portable external validators | `epubcheck` / `qpdf` の `passed` / `failed` / `missing-tool` / `skipped` | tool がある release host では実行し、ない host では `missing-tool` を明示的に扱う |
| Kindle Previewer report contract | fake executable による `validators[]`, `log_path`, failure semantics | required CI の保証として読む |
| Real Kindle Previewer conversion | 実物 Kindle Previewer が生成 artifact を変換できること | Kindle handoff 前に `scripts/validate-real-kindle-previewer.sh` で opt-in 確認する |
| Additional store/device validators | 未実装 | future work として扱う |

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

reference workspace が初期化済みなら、`explain` は current scope の `references/README.md` と `entries/*.md`、`series` では shared scope 側の reference file も structure summary と `--json` に含める。

story workspace が初期化済みなら、`explain` は current scope の `story/README.md`、book scope の `scenes.yml` と `scene-notes/*.md` と `structures/*.md`、entity Markdown、`series` では shared scope 側の story file も structure summary と `--json` に含める。

人向けの summary 出力では、field の意味を追いやすいように config reference への URL も末尾に出す。

editor integration 向けには `--json` も使える。resolved config の要約、主要 field の origin、chapter list、初期化済み reference / story workspace file などを機械可読で返す。

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
  engine: chromium
  toc: true
  page_number: true
  running_header: auto
```

既定では `pdf.toc: true`。ただし `paper` / `conference-preprint` の scaffold では `pdf.toc: false` を書く。`book.writing_mode: vertical-rl` の prose print 既定 engine は `chromium`、`horizontal-ltr` と `conference-preprint` は `weasyprint`。

`pdf.toc: false` にすると、print build では Pandoc の `--toc` を付けずに実行する。

prose の Kindle / EPUB build では `styles/base.css` と `styles/epub.css` を Pandoc に渡す。`series` repo では対応する `shared/styles/base.css`, `shared/styles/epub.css` を使う。

`chromium` を使う print build では `styles/base.css`, `styles/print.css`, generated layout stylesheet を含む self-contained HTML を Pandoc で作り、その HTML を headless Chromium で PDF 化する。`series` repo では対応する `shared/styles/base.css`, `shared/styles/print.css` を使う。縦組み prose print では、この generated layout stylesheet が frontmatter の改ページを持ち、TOC があれば本文は TOC 後の次ページ、TOC がなければ本文は title 後の次ページから始まる。Chromium の margin box 挙動に合わせて、page number は各ページの下中央、running header は有効な場合だけ各ページの上中央へ置き、title / TOC など frontmatter では両方を抑制する。

`weasyprint` を使う print build では `styles/base.css`, `styles/print.css` と、`pdf` / `print` 設定から生成した layout stylesheet を合わせて Pandoc に渡す。`conference-preprint` では A4、余白、2 段組、本文サイズがこの generated stylesheet に反映される。`weasyprint` は `vertical-rl` prose print には使えない。

`typst` では `columns`, `papersize`, `margin`, `fontsize`, `linestretch` の変数として渡す。

## Generated scaffold

`init` は標準では対話式で、作品カテゴリ、`paper` の場合は profile、repo mode、`series` の場合は初期 book id、タイトル、著者名、言語、出力先、本文方向、綴じ方向を確認してから、必要に応じて print 設定、manga 設定、prose の前付き / 後付き、Git、sample 生成も質問する。最後に scaffold plan の summary を表示し、確認後に生成する。`--non-interactive --config-template <template>` を使うと既定値で生成できる。`--title`, `--author`, `--language`, `--output-preset`, `--repo-mode` で対話項目を explicit に上書きできる。`series` では追加で `--initial-book-id <book-id>` を受け付け、既定値は `vol-01` になる。prose の前付き / 後付き scaffold が必要な場合は `--include-introduction`, `--include-afterword` を追加で指定できる。`paper` では追加で `--config-profile paper|conference-preprint` を受け付ける。

テンプレートに応じて、次のような土台を生成する。

- `book.yml` または `series.yml`
- `dist/`
- `.gitignore`, `.gitattributes` (`git.lfs: true` のとき)
- `.agents/skills/shosei-project/SKILL.md`
- `.agents/skills/shosei-content-review/SKILL.md`
- `single-book` では `assets/cover/`, `assets/images/`, `assets/fonts/`, `styles/`
- `series` では `shared/assets/`, `shared/styles/`, `shared/fonts/`, `shared/metadata/`, `books/<book-id>/assets/`
- prose 系では `single-book` に原稿ファイルと `editorial/*.yml`、`series` に `books/<book-id>/manuscript/` と `books/<book-id>/editorial/*.yml`
- manga 系では `single-book` に `manga/`、`series` に `books/<book-id>/manga/`

prose 系テンプレートでは、最初の原稿ファイルとして `paper` / `conference-preprint` は `single-book` で `manuscript/01-main.md`、`series` で `books/<book-id>/manuscript/01-main.md` を生成する。その他の prose は `01-chapter-1.md` を生成する。この `01-` prefix は初期命名の慣例で、章順の source of truth ではない。対話で opt-in した場合だけ、`single-book` では `manuscript/00-introduction.md` と `manuscript/99-afterword.md`、`series` では `books/<book-id>/manuscript/00-introduction.md` と `books/<book-id>/manuscript/99-afterword.md` も追加する。

Kindle を含む scaffold では、`single-book` に `cover.ebook_image: assets/cover/front.png` と `assets/cover/front.png` を生成する。`series` では各巻の `book.yml` に `cover.ebook_image: books/<book-id>/assets/cover/front.png` を書き、対応する placeholder cover asset も `books/<book-id>/assets/cover/front.png` に置く。

また、prose 系では空の `editorial/style.yml`, `editorial/claims.yml`, `editorial/figures.yml`, `editorial/freshness.yml` を生成し、`single-book` では `book.yml`、`series` では `books/<book-id>/book.yml` から参照する。style 側は `single-book` では `styles/base.css`, `styles/epub.css`, `styles/print.css`、`series` では `shared/styles/base.css`, `shared/styles/epub.css`, `shared/styles/print.css` を生成する。これらの default CSS は template/profile ごとに異なり、`init` で選んだ `book.writing_mode` に合わせて prose の `base.css` を切り替える。`novel` / `light-novel` の `print.css` は PDF 向けに本文サイズを半段締め、扉と目次を控えめに整える。本文へのページ分離は generated layout stylesheet が持つ。

生成される config field の意味は [設定リファレンス](config-reference.md) にまとめる。正式な schema や制約は `docs/specs/` を参照する。

`init` 完了メッセージにも config reference への URL を出す。

さらに、`single-book` なら `shosei explain` / `shosei validate`、`series` なら生成された初期 book id 付きの次コマンド例を出す。初期 book id は 1 つの path segment でなければならず、`/`, `\\`, 空白, `.`, `..` は受け付けない。

対話で Git 初期化を選んだ場合は `git init` を実行する。local の `.git` が見つからず Git 初期化を選んでいないときは `git init` を案内し、`git.lfs: true` を選んだときだけ `git lfs install` も案内する。`doctor` を自動実行していないときは `shosei doctor` も続けて案内する。

## Preview and doctor

`preview` は one-shot と `--watch` をサポートする。`--watch` では `book.yml` / `series.yml`、原稿、styles、assets、`shared/` の変更を監視し、変更を検知した path の要約を表示した上で再生成する。再生成失敗時も監視は継続する。

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

`doctor` は required / optional を分けて表示し、PATH 解決結果、バージョン、導入ヒントを返す。初期化済み repo の内側で実行した場合は、検出した repo mode / book / project type / enabled outputs と、その book で重点的に確認すべき tool 群も続けて表示する。

```bash
shosei doctor
shosei doctor --json
```

- `git`
- `pandoc`
- `weasyprint`
- `chromium`
- `typst`
- `lualatex`
- `epubcheck`
- `qpdf`
- `git-lfs`
- Kindle Previewer

required tool は `git`, `pandoc`, `weasyprint`, `chromium`。optional tool は `typst`, `lualatex`, `epubcheck`, `qpdf`, `git-lfs`, Kindle Previewer。

`typst`, `lualatex` は config 値として受け付けるが、v0.2 では default 経路より検証が薄い。doctor では optional tool として表示し、選択中 engine の場合だけ focused required tools に含める。

`--json` は editor integration 向けで、host OS、required / optional ごとの available / missing / pending 件数、各 tool の category / status / path / version / install hint に加えて、検出できた current project の repo mode / book / outputs / focused tools も機械可読で返す。

`handoff` の destination は `kindle`, `print`, `proof` の 3 つに固定する。

```bash
shosei handoff kindle
shosei handoff print
shosei handoff proof
```

- `handoff kindle`: 対応する Kindle artifact、`reports/validate.json`、`manifest.json`、設定済みなら cover asset のコピーを package に含める
- `handoff print`: 対応する print artifact、`reports/validate.json`、`manifest.json` を package に含める
- `handoff proof`: build できた artifact 全件、`reports/validate.json`、`review-notes.md`、`reports/review-packet.json` を package に含める。prose では editorial sidecar もコピーする

`manifest.json` には `build_summary`, `build_stages`, `build_inputs`, `selected_artifacts`, `selected_artifact_details`, `validation_report`, `git_commit`, `git_dirty`, `dirty_worktree_warning` を含める。`proof` では加えて `review_notes`, `review_packet`, `editorial_summary`, `editorial_files` も入る。

`selected_artifact_details` の各 entry には `channel`, `target`, `path`, `primary_tool`, `target_profile`, `artifact_metadata` を含める。`artifact_metadata` には少なくとも次を入れる。

- prose print: `print.pdf_engine`, `toc`, `page_numbering`, `running_header`, `trim_size`, `bleed`, `crop_marks`, `page_margin`, `sides`, `pdf_standard`, `page_count`, `fonts_embedded`
- Kindle: `kindle.fixed_layout`, `kindle.reading_direction`, `kindle.cover_ebook_image`
- manga: `manga.source_page_count`, `rendered_page_count`, `spread_policy_for_kindle`, `unique_page_dimensions`

## Cross-platform smoke

GitHub Actions の CI は `ubuntu-latest`, `macos-latest`, `windows-latest` の 3 OS matrix で動かす。

現在の command-level smoke は次を step 名つきで実行している。

- `shosei init`
- `shosei validate --json`
- `shosei validate --json` の print validator run
- `shosei validate --json` の Kindle Previewer validator run
- `shosei validate`
- `shosei build`
- `shosei preview`
- `shosei chapter add`
- `shosei reference scaffold`
- `shosei reference check`
- `shosei reference drift`
- `shosei reference sync`
- `shosei story seed`
- `shosei page check`
- `shosei series sync`
- `shosei handoff proof`
- `shosei handoff kindle`
- `shosei handoff print`
- `shosei doctor`
- `shosei --help`

合わせて `cargo test --workspace` と `cargo test -p shosei-core --test repo_discovery` も走る。print validator run と Kindle Previewer validator run の smoke は Unix shell fixture を使うため Ubuntu / macOS で回す。VS Code adapter については `npm ci` の後に `npm run check`, `npm test`, `npm run test:host` を別 job で継続確認し、Ubuntu では `npm run test:package-smoke` で package 済み VSIX の smoke も回す。

実物 Kindle Previewer は proprietary / OS-dependent なので required CI には含めない。release operator や maintainer が実物 tool の conversion evidence を残したい場合は、対応 host 上で次を実行する。

```bash
scripts/validate-real-kindle-previewer.sh
```

この hook は一時 workspace に最小構成の Kindle project を作り、`validation.kindle_previewer: true` で `shosei validate --json` を実行する。成功時は `dist/reports/default-validate.json` と `dist/logs/default-kindle-previewer-validate.log` を evidence として確認できる。

CI 表示だけで「どの OS でどの smoke が通ったか」を読める状態を維持し、README と `site/usage.html` でも同じ保証内容を案内する。
