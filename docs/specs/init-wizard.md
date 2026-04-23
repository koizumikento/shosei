# `shosei init` ウィザード仕様 v0.2

作成日: 2026-04-12  
状態: Current

## 1. 目的

`shosei init` は、新規プロジェクトを最小の迷いで立ち上げるための対話式コマンドである。対話で集める値は、必要に応じて explicit な CLI 引数からも渡せる。

このウィザードは次を行う。

- プロジェクト種別の決定
- リポジトリ管理単位の決定
- `book.yml` の初期生成
- 標準ディレクトリ作成
- Git 用ファイルの生成
- 推奨依存の存在確認

## 2. UX 方針

- 引数なし起動を標準とする
- 共通質問を先に集め、category / outputs に応じて分岐質問を追加する
- category と outputs に応じて分岐質問を行う
- 回答後に summary を表示し、最後に確認を求める
- `--non-interactive` は CI 用の例外モードとする

## 3. 基本フロー

1. 実行ディレクトリの確認
2. 既存ファイルの衝突確認
3. プロジェクト概要の質問
4. リポジトリ管理単位の質問
5. 出力 profile の質問
6. レイアウトの質問
7. Git の質問
8. サンプル生成有無の質問
9. 生成内容の summary 表示
10. 確認後にファイル生成
11. 必要なら `doctor` 案内

v0.2 の現行実装は、このフローを次の範囲で満たす。

- 作品カテゴリ
- paper profile
- repo mode
- `series` の場合の初期 book id
- タイトル
- 著者名
- 言語
- 出力先
- 本文方向
- 綴じ方向
- print target / trim size / bleed / crop marks
- `conference-preprint` の場合の sides / max pages
- `manga` の場合の Kindle 見開きポリシー / 巻頭カラー枚数 / 本文ページモード
- prose の場合の `はじめに` / `おわりに` scaffold 有無
- Git 初期化
- Git LFS
- サンプル生成有無
- 生成前 summary 表示
- 最終確認
- `doctor` 実行有無

現時点で future のまま残す項目:

- プロジェクト名を title と別に明示入力する質問
- print margin を個別数値で決める質問
- `light-novel` 専用の画像運用ポリシー分岐
- interactive で選べる layout / print / Git / sample 値を CLI flag でも個別指定する拡張

## 4. 起動パターン

### 標準

```bash
shosei init
```

### パス指定

```bash
shosei init path/to/project
```

### 例外的 override

```bash
shosei init ./my-book --non-interactive --config-template novel --title "My Book" --author "Ken" --language ja --output-preset both
```

v0.2 で残す引数:

- `--non-interactive`
- `--force`
- `--config-template`
- `--repo-mode`
- `--initial-book-id`
- `--title`
- `--author`
- `--language`
- `--output-preset`
- `--include-introduction`
- `--include-afterword`
- positional `PATH`

補足:

- 本文方向、綴じ方向、print 設定、Git、sample 生成は現在 interactive 質問で受ける
- `--include-introduction`, `--include-afterword` は non-interactive で prose の前付き / 後付き scaffold を opt-in するためのフラグとする
- 本文方向、綴じ方向、print 設定、Git、sample 生成は dedicated CLI flag をまだ持たない

## 5. 質問一覧

### 5.1 共通質問

1. 作品カテゴリ
2. `paper` の場合の paper profile
3. リポジトリ管理単位
4. `series` の場合の初期 book id
5. タイトル
6. 著者名
7. 言語
8. 出力先

### 5.2 レイアウト質問

9. 本文方向
10. 綴じ方向
11. print を含む prose の場合の print target
12. print を含む prose の場合の trim size
13. print を含む prose の場合の bleed
14. print を含む prose の場合の crop marks
15. `conference-preprint` の場合の印刷面
16. `conference-preprint` の場合の最大ページ数

### 5.3 manga 質問

17. `manga` の場合の Kindle 見開きポリシー
18. `manga` の場合の巻頭カラー枚数
19. `manga` の場合の本文ページモード

### 5.4 Git 質問

22. Git リポジトリを初期化するか
23. Git LFS を使う前提にするか

### 5.5 prose 補助質問

20. 前付きを追加するか
21. 後付きを追加するか

### 5.6 サンプル質問

24. サンプル原稿を生成するか
25. 実行後に `shosei doctor` を走らせるか

## 6. 分岐ルール

### `project.type = business`

- 既定 `writing_mode = horizontal-ltr`
- 既定 `binding = left`
- 既定 profile は `business`
- 必要なら `00-introduction.md` と `99-afterword.md` を opt-in で追加できる
- サンプルは本文 + 図表ダミー画像

### `project.type = paper`

- 既定 `writing_mode = horizontal-ltr`
- 既定 `binding = left`
- 既定 profile は `paper`
- print を含む scaffold では `writing_mode = horizontal-ltr` なら `pdf.engine = weasyprint`、`vertical-rl` なら `chromium`
- 必要なら `00-introduction.md` と `99-afterword.md` を opt-in で追加できる
- 追加質問:
  - `paper`
  - `conference-preprint`
- `conference-preprint` では print を既定出力に寄せ、A4 / 2 段組 / 両面の preset を提示し、`pdf.engine = weasyprint` を維持する

### `project.type = novel`

- 既定 `writing_mode = vertical-rl`
- 既定 `binding = right`
- 既定 profile は `novel`
- print を含む scaffold では `pdf.engine = chromium`
- scaffold する `styles/print.css` は PDF 向けに本文 10.5pt を既定とし、扉と目次を本文より控えめに整える
- 必要なら `00-introduction.md` と `99-afterword.md` を opt-in で追加できる
- サンプルは `prologue`, `chapter`, `colophon`

### `project.type = light-novel`

- 既定 `writing_mode = vertical-rl`
- 既定 `binding = right`
- 既定 profile は `light-novel`
- print を含む scaffold では `pdf.engine = chromium`
- scaffold する `styles/print.css` は PDF 向けに本文 10pt を既定とし、扉と目次を本文より控えめに整える
- 必要なら `00-introduction.md` と `99-afterword.md` を opt-in で追加できる
- 画像方針確認は future 拡張に残す

### `project.type = manga`

- 既定 `writing_mode = vertical-rl`
- 既定 `binding = right`
- 既定 profile は `manga`
- prose 用 `manuscript/` ではなく `manga/` 主体で生成
- 追加質問:
  - Kindle 向け見開き劣化ポリシー
  - 巻頭カラーページ数
  - 本文ページモード

## 7. 出力先ごとの分岐

### `kindle`

追加/確認項目:

- `reading_direction`
- Kindle target
  - prose: `kindle-ja`
  - manga: `kindle-comic`
- Kindle を含む scaffold では `cover.ebook_image` と placeholder cover asset も初期生成する

### `print`

追加/確認項目:

- PDF profile
  - `print-jp-pdfx1a`
  - `print-jp-pdfx4`
- trim size
- bleed
- crop marks
- `project.type = paper` かつ `conference-preprint` の場合は duplex, max pages

補足:

- 現行実装では margin を個別入力させず、必要な値は profile 既定または generated stylesheet 側で補う
- `conference-preprint` では A4 / 2 段組 / 既定 margin を generated stylesheet へ反映する

### `both`

- `kindle` と `print` の両質問を行う
- prose では layout 質問の後に前付き / 後付きの opt-in 質問を行う

## 8. 詳細質問定義

### 8.1 作品カテゴリ

- Prompt: `作品カテゴリを選んでください`
- Choices:
  - `business`
  - `paper`
  - `novel`
  - `light-novel`
  - `manga`

### 8.2 リポジトリ管理単位

- Prompt: `このリポジトリを何単位で管理しますか`
- Choices:
  - `single-book`
  - `series`
- Default:
  - `business`: `single-book`
  - `paper`: `single-book`
  - `novel`: `single-book`
  - `light-novel`: `single-book`
  - `manga`: `series`

補足:

- `single-book`: 1 冊または 1 巻を 1 repo として扱う
- `series`: シリーズ全体を 1 repo にまとめ、各巻を子ディレクトリで持つ
- `series` を選んだ場合だけ初期 book id を質問する
- 初期 book id の既定値は `vol-01`
- 初期 book id は 1 つの path segment とし、空文字、`/`, `\\`, 空白, `.`, `..` は受け付けない

### 8.3 言語

- Prompt: `本文の言語コードを入力してください`
- Default: `ja`
- Validation:
  - 空文字不可
  - BCP 47 に近い形式を warning 付きでチェック

### 8.4 本文方向

- Prompt: `本文方向を選んでください`
- Choices:
  - `horizontal-ltr`
  - `vertical-rl`
- Default:
  - `business`: `horizontal-ltr`
  - `paper`: `horizontal-ltr`
  - それ以外: `vertical-rl`

### 8.5 綴じ方向

- Prompt: `綴じ方向を選んでください`
- Choices:
  - `right`
  - `left`
- Default:
  - `vertical-rl`: `right`
  - `horizontal-ltr`: `left`

### 8.6 判型

- Prompt: `判型を選んでください`
- Choices:
  - `A4`
  - `A5`
  - `B6`
  - `bunko`

### 8.7 paper profile

`project.type = paper` のときのみ質問する。

- Prompt: `論文 profile を選んでください`
- Choices:
  - `paper`
  - `conference-preprint`
- Default: `paper`

### 8.8 Kindle 見開きポリシー

`project.type = manga` のときのみ質問する。

- Prompt: `Kindle で見開きをどう扱いますか`
- Choices:
  - `split`
  - `single-page`
  - `skip`
- Default: `split`

### 8.9 カラーページ

`project.type = manga` のときのみ質問する。

- Prompt: `巻頭カラーページ数を入力してください`
- Default: `0`
- Validation:
  - 0 以上の整数

### 8.10 Git LFS

- Prompt: `画像や作画データを Git LFS 対象として設定しますか`
- Default: `yes`

## 9. 生成ファイル

### 共通

- `book.yml` または `series.yml`
- `dist/`
- `.gitignore`
- `.gitattributes` (`git.lfs = true` のとき)
- `.agents/skills/shosei-project/SKILL.md`
- `.agents/skills/shosei-content-review/SKILL.md`
- `single-book` では `assets/cover/`, `assets/images/`, `assets/fonts/`, `styles/`
- `series` では `shared/assets/`, `shared/styles/`, `shared/fonts/`, `shared/metadata/`

### prose

- `single-book` では `paper` / `conference-preprint` に `manuscript/01-main.md`、それ以外に `manuscript/01-chapter-1.md`
- `single-book` では `editorial/style.yml`, `editorial/claims.yml`, `editorial/figures.yml`, `editorial/freshness.yml`
- `single-book` では `styles/base.css`, `styles/epub.css`, `styles/print.css`
- `series` では `paper` / `conference-preprint` に `books/<book-id>/manuscript/01-main.md`、それ以外に `books/<book-id>/manuscript/01-chapter-1.md`
- `series` では `books/<book-id>/editorial/style.yml`, `books/<book-id>/editorial/claims.yml`, `books/<book-id>/editorial/figures.yml`, `books/<book-id>/editorial/freshness.yml`
- `series` では `shared/styles/base.css`, `shared/styles/epub.css`, `shared/styles/print.css`
- Kindle を含む scaffold では `single-book` に `cover.ebook_image: assets/cover/front.png` と `assets/cover/front.png`、`series` に `cover.ebook_image: books/<book-id>/assets/cover/front.png` と `books/<book-id>/assets/cover/front.png`
- prose の style file は template/profile ごとの既定見た目を持つ
  - `business`, `paper`: 横組み prose 向け
  - `novel`, `light-novel`: 縦組み prose 向け
    - `print.css` では PDF 用の本文サイズを半段締め、扉と目次を base / EPUB より控えめにする
  - `conference-preprint`: `paper` 系 style を継承し、強い layout 差分は config-generated print stylesheet 側で表す
- `conference-preprint` では `book.yml` に A4 / 2 段組 / 両面 preset を出力する

### manga

- `manga/script/`
- `manga/storyboard/`
- `manga/pages/`
- `manga/spreads/`
- `manga/metadata/`

### `repo_mode = series`

- `series.yml`
- `books/<book-id>/book.yml`
- `books/<book-id>/assets/`
- prose では `books/<book-id>/manuscript/`
- manga では `books/<book-id>/manga/`
- `<book-id>` は質問または `--initial-book-id` で決め、未指定時は `vol-01` を使う

### `generate_sample = false`

- prose では空の初期原稿ファイルを生成する
- manga では追加サンプルファイルは生成しない

### `generate_sample = true`

追加生成:

- prose: サンプル章本文
- light-novel: サンプル挿絵参照を含む本文
- manga: 追加サンプルファイルは生成せず、空ディレクトリ scaffold に留める

## 10. `book.yml` 生成ルール

- `single-book` では回答を `book.yml` に反映する
- `series` ではシリーズ共通項目を `series.yml` に、巻固有項目を `books/<book-id>/book.yml` に振り分ける
- `series` の `<book-id>` は初期 book id の回答値または `--initial-book-id` を使い、未指定時は `vol-01` を使う
- 未回答だが既定値がある項目は明示出力する
- 将来用だが未実装の項目は必要最小限に留める
- 生成する YAML config は簡潔さを優先し、field の意味は別の config reference にまとめる
- prose では `manuscript` を生成
- manga では `manga` を生成し、`manuscript` は省略

## 11. `.gitignore` 生成ルール

最低限以下を含める。

```gitignore
dist/
.DS_Store
*.log
```

将来キャッシュディレクトリを導入した場合はここに追加する。

## 12. `.gitattributes` 生成ルール

`git.lfs = true` のとき、以下を初期出力候補とする。

```gitattributes
*.psd filter=lfs diff=lfs merge=lfs -text lockable
*.clip filter=lfs diff=lfs merge=lfs -text lockable
*.kra filter=lfs diff=lfs merge=lfs -text lockable
*.tif filter=lfs diff=lfs merge=lfs -text lockable
*.png filter=lfs diff=lfs merge=lfs -text
*.jpg filter=lfs diff=lfs merge=lfs -text
```

## 12.1 エージェントスキルテンプレート

`shosei init` は repo-scoped な agent skill templates を 2 つ生成する。

ルール:

- 出力先は `.agents/skills/shosei-project/SKILL.md` と `.agents/skills/shosei-content-review/SKILL.md`
- `SKILL.md` は instruction-first を既定とし、`scripts/`, `references/`, `agents/openai.yaml` は生成しない
- frontmatter の `description` は、何をする skill かと、いつ使うかを両方書く
- `shosei-project` の本文には少なくとも次を含める
  - `single-book` / `series` の判定方法
  - `series` repo での `--book <book-id>` 利用ルール
  - `shosei explain` を先に使う方針
  - `validate`, `build`, `preview`, `handoff` の基本導線
  - config path を repo-relative かつ `/` 区切りで保つルール
- `shosei-content-review` の本文には少なくとも次を含める
  - manuscript, editorial, story, reference, proof packet を対象にすること
  - reference workspace がある場合は `reference map` を先に使い、source-backed review では reference entry を主要な review aid として扱うこと
  - `series` で reference を使う review では book-scoped と shared の scope を見分け、必要なら `reference drift` で source of truth の衝突を確認すること
  - findings-first で内容上の問題や review readiness を見ること
  - コードレビューや CLI 実装レビューではないこと
  - rewrite ではなく指摘を返すこと
- init 時点の `project.type` と `repo_mode` を両 skill に埋め込み、利用者があとから project 固有メモを追記できる形にする

## 13. 実行前チェック

ファイル生成前に次を確認する。

- 出力先パスが存在するか
- 既存 `book.yml` がないか
- 既存ファイルと衝突しないか
- `--force` なしで上書きが起きないか

## 14. 実行後チェック

`shosei init` 完了後、必要に応じて以下を案内する。

- field の意味を追うための config reference URL
- repo mode に応じた `shosei explain` / `shosei validate` の次コマンド例
- 必要時だけ `git init`
- Git LFS 未設定マシン向けの `git lfs install`
- `run doctor = no` のときだけ `shosei doctor`
- 初回 commit

`run doctor = yes` を選んだ場合は自動実行してもよい。

## 15. エラー方針

- 既存ファイル衝突は error
- 不正 enum は再入力
- 必須文字列が空なら再入力
- 非対話モードで必須値不足なら即 failure

メッセージは以下の構成にする。

- 何が足りないか
- どの値が許可されるか
- 修正例

## 16. summary 表示

生成直前に次を表示する。

- project type
- repo mode
- initial book id (`series` のときのみ)
- title
- author
- writing mode
- binding
- outputs
- print profile
- git enabled
- git lfs enabled
- sample enabled
- target path

例:

```text
Project summary
  type: light-novel
  repo_mode: series
  initial_book_id: vol-01
  title: 作品名
  writing_mode: vertical-rl
  binding: right
  outputs: kindle, print
  print.target: print-jp-pdfx1a
  git: enabled
  git-lfs: enabled
```

## 17. 非対話モード

v0.2 では CI や editor integration からの scaffold 生成にも使う。

非対話モードでは template ごとの既定値を使って scaffold を生成し、次の値だけを explicit に override できる。

- `config-template`
- `repo-mode`
- `initial-book-id`
- `title`
- `author`
- `language`
- `output-preset`

未指定の項目は template に応じた既定値にフォールバックする。

## 18. 将来拡張

- `init --from-template`
- `init --from-example`
- 既存 repo への後付け `shosei adopt`
- `shosei migrate --to series`
- printer preset 選択
- Kindle Previewer 検出と自動連携
