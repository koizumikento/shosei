# `shosei init` ウィザード仕様 v0.1

作成日: 2026-04-12  
状態: Draft

## 1. 目的

`shosei init` は、新規プロジェクトを最小の迷いで立ち上げるための対話式コマンドである。

このウィザードは次を行う。

- プロジェクト種別の決定
- リポジトリ管理単位の決定
- `book.yml` の初期生成
- 標準ディレクトリ作成
- Git 用ファイルの生成
- 推奨依存の存在確認

## 2. UX 方針

- 引数なし起動を標準とする
- 質問数は 10 前後に抑える
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

v0.1 の現行実装は、このフローのうち次を先に満たす。

- 作品カテゴリ
- repo mode
- paper profile
- タイトル
- 著者名
- 言語
- 出力先
- `doctor` 実行有無

判型や PDF profile などの分岐質問は後続拡張とする。

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
shosei init --non-interactive --config-template novel
```

v0.1 で残す引数は最小限:

- `--non-interactive`
- `--force`
- `--path`
- `--config-template`
- `--repo-mode`

## 5. 質問一覧

### 5.1 共通質問

1. プロジェクト名
2. 作品カテゴリ
3. リポジトリ管理単位
4. タイトル
5. 著者名
6. 言語
7. 出力先

### 5.2 レイアウト質問

8. 本文方向
9. 綴じ方向
10. 判型

### 5.3 Git 質問

11. Git リポジトリを初期化するか
12. Git LFS を使う前提にするか

### 5.4 サンプル質問

13. サンプル原稿を生成するか
14. 実行後に `shosei doctor` を走らせるか

## 6. 分岐ルール

### `project.type = business`

- 既定 `writing_mode = horizontal-ltr`
- 既定 `binding = left`
- 既定 profile は `business`
- サンプルは本文 + 図表ダミー画像

### `project.type = paper`

- 既定 `writing_mode = horizontal-ltr`
- 既定 `binding = left`
- 既定 profile は `paper`
- 追加質問:
  - `paper`
  - `conference-preprint`
- `conference-preprint` では print を既定出力に寄せ、A4 / 2 段組 / 両面の preset を提示する

### `project.type = novel`

- 既定 `writing_mode = vertical-rl`
- 既定 `binding = right`
- 既定 profile は `novel`
- サンプルは `prologue`, `chapter`, `colophon`

### `project.type = light-novel`

- 既定 `writing_mode = vertical-rl`
- 既定 `binding = right`
- 既定 profile は `light-novel`
- 画像方針確認を追加
  - `full-page`
  - `spread`

### `project.type = manga`

- 既定 `writing_mode = vertical-rl`
- 既定 `binding = right`
- 既定 profile は `manga`
- prose 用 `manuscript/` ではなく `manga/` 主体で生成
- 追加質問:
  - カラーページ有無
  - Kindle 向け見開き劣化ポリシー

## 7. 出力先ごとの分岐

### `kindle`

追加/確認項目:

- `reading_direction`
- Kindle target
  - prose: `kindle-ja`
  - manga: `kindle-comic`

### `print`

追加/確認項目:

- PDF profile
  - `print-jp-pdfx1a`
  - `print-jp-pdfx4`
- trim size
- page margins
- bleed
- crop marks
- `project.type = paper` かつ `conference-preprint` の場合は column layout, duplex, max pages

### `both`

- `kindle` と `print` の両質問を行う

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
  - `custom`

### 8.7 paper profile

`project.type = paper` のときのみ質問する。

- Prompt: `論文 profile を選んでください`
- Choices:
  - `paper`
  - `conference-preprint`
- Default: `paper`

### 8.8 Kindle 見開きポリシー

`project.type = light-novel | manga` のかつ `kindle` 有効時のみ質問する。

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
- Default:
  - `manga`, `light-novel`: `yes`
  - それ以外: `yes`

## 9. 生成ファイル

### 共通

- `book.yml` または `series.yml`
- `dist/`
- `.gitignore`
- `.gitattributes`
- `.agents/skills/shosei-project/SKILL.md`
- `single-book` では `assets/cover/`, `assets/images/`, `assets/fonts/`, `styles/`
- `series` では `shared/assets/`, `shared/styles/`, `shared/fonts/`, `shared/metadata/`

### prose

- `single-book` では `manuscript/01-chapter-1.md`
- `single-book` では `editorial/style.yml`, `editorial/claims.yml`, `editorial/figures.yml`, `editorial/freshness.yml`
- `single-book` では `styles/base.css`, `styles/epub.css`, `styles/print.css`
- `series` では `books/<book-id>/manuscript/01-chapter-1.md`
- `series` では `books/<book-id>/editorial/style.yml`, `books/<book-id>/editorial/claims.yml`, `books/<book-id>/editorial/figures.yml`, `books/<book-id>/editorial/freshness.yml`
- `series` では `shared/styles/base.css`
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
- `books/<book-id>/manuscript/` または `books/<book-id>/manga/`

### `--sample` 相当

追加生成:

- prose: サンプル章本文
- light-novel: サンプル挿絵参照
- manga: サンプル `page-manifest.yml`

## 10. `book.yml` 生成ルール

- `single-book` では回答を `book.yml` に反映する
- `series` ではシリーズ共通項目を `series.yml` に、巻固有項目を `books/<book-id>/book.yml` に振り分ける
- 未回答だが既定値がある項目は明示出力する
- 将来用だが未実装の項目は必要最小限に留める
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

`shosei init` は repo-scoped な agent skill template を生成する。

ルール:

- 出力先は `.agents/skills/shosei-project/SKILL.md`
- `SKILL.md` は instruction-first を既定とし、`scripts/`, `references/`, `agents/openai.yaml` は生成しない
- frontmatter の `description` は、何をする skill かと、いつ使うかを両方書く
- 本文には少なくとも次を含める
  - `single-book` / `series` の判定方法
  - `series` repo での `--book <book-id>` 利用ルール
  - `shosei explain` を先に使う方針
  - `validate`, `build`, `preview`, `handoff` の基本導線
  - config path を repo-relative かつ `/` 区切りで保つルール
- init 時点の `project.type` と `repo_mode` を skill に埋め込み、利用者があとから project 固有メモを追記できる形にする

## 13. 実行前チェック

ファイル生成前に次を確認する。

- 出力先パスが存在するか
- 既存 `book.yml` がないか
- 既存ファイルと衝突しないか
- `--force` なしで上書きが起きないか

## 14. 実行後チェック

`shosei init` 完了後、必要に応じて以下を案内する。

- `shosei doctor`
- `git init`
- `git lfs install`
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
  title: 作品名
  writing_mode: vertical-rl
  binding: right
  outputs: kindle, print
  print.target: print-jp-pdfx1a
  git: enabled
  git-lfs: enabled
```

## 17. 非対話モード

v0.1 では CI やテンプレート生成用途に限定する。

最低限必要:

- `project.type`
- `book.title`
- `book.authors`
- `outputs`

不足時は failure にする。

## 18. 将来拡張

- `init --from-template`
- `init --from-example`
- 既存 repo への後付け `shosei adopt`
- `shosei migrate --to series`
- printer preset 選択
- Kindle Previewer 検出と自動連携
