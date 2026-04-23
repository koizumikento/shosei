# 電子書籍制作 CLI 機能仕様書 v0.2

作成日: 2026-04-12  
状態: Current

## 1. 目的

本ツールは、日本語書籍制作を対象にした CLI ツールである。対象フォーマットは `EPUB` と `PDF` を基本とし、配布先としては次を優先する。

- Kindle 向け電子書籍
- 日本の印刷会社向け入稿データ

また、通常の文章主体の書籍だけでなく、漫画制作の工程も同じプロジェクト内で管理できることを目指す。

本ツールは単なる変換器ではなく、以下を一貫して扱う。

- 書籍プロジェクトの初期化
- 原稿構成の管理
- 出力 profile に応じたビルド
- 検証と品質ゲート
- editorial metadata に基づく review readiness
- 納品用成果物の取りまとめ
- Git 前提の履歴管理
- macOS / Windows / Linux で動作する単一 CLI の提供

## 2. スコープ

### 対象カテゴリ

- ビジネス書
- 論文
- 小説
- ライトノベル
- 漫画

### 対象出力

- `kindle-ja`: Kindle 日本語向け EPUB
- `print-jp-pdfx1a`: 日本の印刷会社向け PDF/X-1a
- `print-jp-pdfx4`: 日本の印刷会社向け PDF/X-4

### 非目標

- WYSIWYG エディタの提供
- ストアへの直接アップロード
- DTP アプリの完全代替
- 漫画の作画そのもの
- PDF を主原稿とした編集フロー

## 3. 設計原則

- `設定ファイル中心`: 日常操作で多量の CLI 引数を要求しない
- `init 重視`: まず正しいプロジェクト構成を作る
- `profile 駆動`: 出力先ごとの差分は profile に閉じ込める
- `優しいインタフェース`: build 失敗時は修正しやすい形で原因を示す
- `Git first`: 原稿と制作物の履歴を Git で管理する
- `カテゴリ別原稿モデル`: 文章書籍と漫画で入力モデルを分ける
- `Rust 実装`: 単一バイナリ配布と移植性を優先する
- `Cross-platform`: macOS / Windows / Linux で同一コマンド体系を維持する

CLI バイナリ名は `shosei` とする。

## 4. アーキテクチャ方針

### 4.1 文章書籍

`business`, `paper`, `novel`, `light-novel` は Pandoc を中核変換エンジンとして扱う。

- EPUB: Pandoc EPUB3 writer を利用
- PDF:
  - `book.writing_mode: horizontal-ltr` と `conference-preprint` は Pandoc + `weasyprint`
  - `book.writing_mode: vertical-rl` は Pandoc で self-contained HTML を生成し、headless `chromium` で PDF 化する
    - generated page style は Chromium の margin box 挙動に合わせて中央寄せを既定にする
      - page number は各ページの下中央に置く
      - running header は有効な場合だけ各ページの上中央に置く
      - title / TOC など frontmatter では page number と running header を出さない
- ツール本体の責務:
  - プロジェクト構成管理
  - profile 解決
  - アセット解決
  - 事前/事後検証
  - エラー整形

### 4.2 漫画

`manga` はページ画像主体の別原稿モデルとして扱う。

- 単位: volume / chapter / page / spread
- 生成物:
  - Kindle 向け固定レイアウト成果物
  - 印刷会社向け本文 PDF
- 作画データ、ページ画像、見開き指定、カラーページ管理を含む

### 4.3 実装基盤

本ツール本体は Rust で実装する。

詳細な crate 構成と責務分離は [Rust 実装アーキテクチャ](rust-architecture.md) を参照する。

方針:

- 配布物は OS ごとの単一ネイティブバイナリを基本とする
- シェルスクリプト依存ではなく、Rust から外部コマンドを実行する
- パス解決、設定読込、検証、ログ出力は Rust 側で統一的に扱う
- OS ごとの差異はプロセス起動、実行ファイル名、ファイルシステム差異の吸収に閉じ込める

対応対象:

- macOS
- Windows
- Linux

非機能要件:

- 同じ `shosei` コマンド体系が 3 OS で成立すること
- シェル固有構文を前提にしないこと
- パス区切りの違いを内部で吸収すること
- UTF-8 を含むファイル名を扱えること

### 4.4 Editor integration

VS Code 拡張のような editor integration は追加してよいが、build / validate / explain などの実処理は `shosei` CLI に委譲する。

方針:

- editor 側で repo discovery、config merge、pipeline planning を複製しない
- `validate` / `page check` の既存 report を diagnostics 連携に使ってよい
- editor integration の詳細は [VS Code 拡張仕様](vscode-extension.md) を参照する

## 5. 想定ユーザー

- 技術書・ビジネス書の著者
- 論文・前刷りを作る学生、研究者、発表者
- 小説・ライトノベルの制作者
- 漫画制作者、同人制作者、小規模出版社
- 入稿担当、組版担当、外注先とのやりとりを行う人

## 6. プロジェクト構成

リポジトリ管理単位の詳細は [リポジトリ管理モデル](repository-model.md) を参照する。

初期構成の標準形は以下とする。`project.type` に応じて prose (`editorial/` + `manuscript/`) か manga (`manga/`) のどちらかを生成し、`references/` と `story/` workspace は必要なときだけ明示 command で追加する。

```text
project/
  book.yml
  [prose] editorial/
    style.yml
    claims.yml
    figures.yml
    freshness.yml
  .agents/
    skills/
      shosei-project/
        SKILL.md
      shosei-content-review/
        SKILL.md
  [opt-in] references/
  [opt-in] story/
  [prose] manuscript/
    01-chapter-1.md
  [manga] manga/
    script/
    storyboard/
    pages/
    spreads/
    metadata/
  assets/
    cover/
    images/
    fonts/
  styles/
    base.css
    epub.css
    print.css
  dist/
  .gitignore
  .gitattributes
```

補足:

- `manuscript/` は文章書籍向け
- `manga/` は漫画向け
- `editorial/` は prose 系で表記、根拠、図表、鮮度管理を置く
- `references/` は参考リンクと作業メモを置く opt-in workspace
- `story/` は物語補助を使う場合だけ `shosei story scaffold` で追加する opt-in workspace
- `assets/cover/` の画像は外部カバーアセットとし、本文 frontmatter とは分離する
- 実際に使わないディレクトリは空でもよい

## 7. コアコマンド

### 7.1 `shosei init`

対話式でプロジェクトを初期化する。

詳細仕様は [init ウィザード仕様](init-wizard.md) を参照する。

主な責務:

- プロジェクト種別の選択
- 設定ファイル生成
- 標準ディレクトリ作成
- `.gitignore`, `.gitattributes` の作成
- repo-scoped agent skill templates の生成
- Git リポジトリ初期化
- 依存チェック案内

v0.2 の現行質問項目:

1. 作品カテゴリ: `business | paper | novel | light-novel | manga`
2. `paper` の場合の profile: `paper | conference-preprint`
3. リポジトリ管理単位: `single-book | series`
4. `series` の場合の初期 book id
5. タイトル
6. 著者名
7. 言語
8. 出力先: `kindle | print | both`
9. 本文方向: `horizontal-ltr | vertical-rl`
10. 綴じ方向: `left | right`
11. print を含む prose の場合の print target: `print-jp-pdfx1a | print-jp-pdfx4`
12. print を含む prose の場合の trim size: `A4 | A5 | B6 | bunko`
13. print を含む prose の場合の bleed
14. print を含む prose の場合の crop marks 有無
15. `conference-preprint` の場合の印刷面: `simplex | duplex`
16. `conference-preprint` の場合の最大ページ数
17. `manga` の場合の Kindle 見開きポリシー: `split | single-page | skip`
18. `manga` の場合の巻頭カラー枚数
19. `manga` の場合の本文ページモード: `monochrome | mixed | color`
20. prose の場合に前付きを追加するか
21. prose の場合に後付きを追加するか
22. Git リポジトリを初期化するか
23. Git LFS を前提にするか
24. サンプル原稿を生成するか
25. 実行後に `doctor` を流すか

補足:

- `--non-interactive --config-template <template>` を使うと既定値で scaffold を生成できる
- `--title`, `--author`, `--language`, `--output-preset`, `--repo-mode` を付けると対話で決める値を明示 override できる
- prose の前付き / 後付き scaffold は `--include-introduction`, `--include-afterword` で non-interactive でも opt-in できる
- `series` を選ぶ場合は `--initial-book-id <book-id>` も使え、既定値は `vol-01` とする
- 初期 book id は 1 つの path segment とし、空文字、`/`, `\\`, 空白, `.`, `..` は受け付けない
- `paper` を選んだ場合は prose 系のまま扱い、`paper` または `conference-preprint` の profile を後続質問で選べるようにする
- prose では `init` 対話で前付き / 後付きの scaffold 有無を選べるようにし、不要なら `manuscript.frontmatter` / `manuscript.backmatter` を書かない
- Kindle を含む scaffold では `cover.ebook_image` と placeholder cover asset を初期生成し、`init` 直後の `validate` が cover 未設定 warning から始まらないようにする
- prose project では `editorial/style.yml`, `claims.yml`, `figures.yml`, `freshness.yml` も scaffold に含める
- prose project では template/profile が所有する `base.css`, `epub.css`, `print.css` を scaffold し、`series` では `shared/styles/` に置く
  - `novel`, `light-novel` の `print.css` は PDF 向けに本文サイズを半段締め、扉と目次まわりの見た目を整える
  - build-generated print stylesheet は vertical prose print の frontmatter pagination を持つ
    - TOC がある場合は title と TOC を同じ前付けに保ったまま本文だけを次ページに送る
    - TOC がない場合は title の後で本文へ入る前に改ページする
    - page style は Chromium の margin box 挙動に合わせて中央寄せを既定にする
      - page number は各ページの下中央に置く
      - running header は有効な場合だけ各ページの上中央に置く
      - title / TOC など frontmatter では page number と running header を出さない
- 対話で Git 初期化を選んだ場合は `git init` を実行する
- `git.lfs: true` を選んだ場合だけ `.gitattributes` を生成し、完了メッセージでも `git lfs install` を案内する

### 7.2 `shosei build`

`book.yml` を読み、有効な target profile の成果物を生成する。

原則:

- 引数なしで実行できる
- 個別指定は例外的に `--target kindle|print` など最小限に留める
- prose と manga でパイプラインを切り替える

### 7.3 `shosei explain`

解決済み設定と値の由来を表示する。

主な責務:

- repo mode と対象 book の表示
- 最終有効設定の表示
- 各値が `book.yml`、`series.yml` の `defaults`、または built-in default のどれで決まったかの表示
- `series` の `shared.*` 探索パスの表示
- editorial sidecar の参照先と件数の表示
- 初期化済み reference workspace の scope と entry 一覧の表示
- editor integration 向けに `--json` で機械可読 snapshot を返せること

v0.2 の最小要件:

- text 出力でよい
- `--json` で title / type / outputs / origins / structure を返せる
- `--json` の structure には、初期化済み reference workspace の `README.md` と `entries/*.md` を含めてよい
- `single-book` / `series` の両方に対応する
- prose / manga の差分設定を表示する

### 7.4 `shosei validate`

原稿・設定・成果物の検証を行う。

`validate` は単なる lint ではなく、提出前の preflight として振る舞う。

原則:

- 有効な出力 target 全体を既定対象にする
- 例外的に `--target kindle|print` で個別実行できる
- 人間向け summary と機械可読レポートを両方出せる
- v0.2 の既定経路では `dist/reports/<book-id>-validate.json` を更新する
- remediation 後は `--json` で同じ report schema を stdout にも出せる
- target ごとの外部 validator は、対応 artifact と tool が揃う場合は実行する
- 外部 validator を実行できない場合も、missing tool / skipped validator を report に残す
- 外部 validator の詳細ログは `dist/logs/` に保存し、report から参照できる
- Kindle Previewer は proprietary / OS-dependent な tool なので、`validation.kindle_previewer: true` のときだけ optional device-oriented validator として実行する

validator confidence は次の層で扱う。

| Layer | Validator evidence | CI expectation | User / maintainer expectation |
|---|---|---|---|
| Local structural checks | config / preflight / manuscript / target-profile checks | workspace tests と CLI smoke で継続確認する | 標準 validate として常に読む |
| Portable external validators | `epubcheck` / `qpdf` の `passed` / `failed` / `missing-tool` / `skipped` report | fake または install 済み tool fixture で report contract を確認する | tool があれば delivery 前に実行し、なければ `missing-tool` を確認する |
| Kindle device-oriented contract | fake Kindle Previewer executable による `validators[]` schema、log path、failure semantics | real proprietary binary は要求せず、fake executable smoke で継続確認する | `validation.kindle_previewer: true` を明示したときだけ有効にする |
| Real Kindle Previewer execution | 実物 Kindle Previewer による conversion check と `dist/logs/<book-id>-kindle-previewer-validate.log` | required CI にはしない | maintainer / release operator が local hook で必要時に確認する |
| Store/device-specific validators beyond Kindle Previewer | 未定義 | CI 対象外 | future work として扱う |

- 共通 lint
- prose editorial lint
- build に必要なツールの事前確認
- EPUB 検証
- Kindle 想定検証
- 印刷想定検証
- 機械可読レポート出力

prose editorial lint の対象:

- style guide に基づく推奨表記と禁止語
- figure ledger の asset / source / manuscript 参照整合
- claim ledger の section / source 整合
- freshness ledger の参照整合と review due

### 7.5 `shosei preview`

レイアウト確認用のプレビューを生成または起動する。

実行モード:

- one-shot: 現在状態の preview を生成または起動する
- watch: 原稿・設定・styles・assets の変更を監視し、preview を再生成する

追加要件:

- `watch` は macOS / Windows / Linux で同じコマンド体系を保つ
- shell 固有の file watch 構文に依存しない
- 再生成失敗時も監視プロセスは継続し、差分修正を試しやすくする

v0.2 の最小要件:

- one-shot を先に実装する
- `--target kindle|print` を受け付けられる
- 生成した preview 成果物のパスを端末に表示する
- `--watch` を受け付け、原稿・設定・styles・assets・shared の変化で再生成できる
- `--watch` は変更を検知した path の要約を端末に表示する
- 再生成失敗時も、どの変更を起点に失敗したかを端末に表示する

確認対象:

- 縦書き/横書き
- 柱・ノンブル
- 章扉
- 画像の回り込み、全ページ、見開き
- 改ページ

### 7.6 `shosei doctor`

外部依存と環境の確認を行う。

対象例:

- `git`
- `pandoc`
- `weasyprint`
- `chromium`
- `epubcheck`
- `qpdf`
- Kindle Previewer
- `git-lfs`

追加要件:

- macOS / Windows / Linux で実行ファイル名の差異を吸収して検出する
- PATH 上の解決結果とバージョンを表示する
- 不足依存の導入案内を出せるようにする

v0.2 の最小要件:

- required tool と optional tool を分けて返せる
- required tool は `git`, `pandoc`, `weasyprint`, `chromium` とする
- optional tool は `typst`, `lualatex`, `epubcheck`, `qpdf`, `git-lfs`, Kindle Previewer とする
- `typst`, `lualatex` は config 値では受け付けるが、v0.2 では default 経路より検証が薄い。doctor では optional tool として表示し、選択中 engine の場合だけ focused required tools に含める
- PATH 解決結果、バージョン、導入ヒントを text 出力で返せる
- editor integration 向けに machine-readable な `--json` 出力を返せる
- 初期化済み repo の内側で実行した場合は、検出できた current book の project type / enabled outputs と、その book で特に重要な tool 群を追加表示できる
- OS 別の詳細導入案内は将来拡張でよい

### 7.7 `shosei handoff`

提出先に応じた成果物パッケージを生成する。

- `shosei handoff kindle`
- `shosei handoff print`
- `shosei handoff proof`

内容:

- 本体成果物
- 仕様サマリ
- build 情報
- commit 情報
- proof 向け review note

### 7.8 `shosei chapter`

prose project の source structure を更新する。

- `shosei chapter add <path>`
- `shosei chapter move <path>`
- `shosei chapter remove <path>`
- `shosei chapter renumber`

主な責務:

- `project.type != manga` の book に対して `manuscript.chapters` を更新する
- `series` では既存の repo discovery に従って対象 book を解決する
- 章ファイルの追加時に必要なら Markdown stub を生成する
- 削除対象ファイルに対応する `sections` entry があれば整合のために除去する
- 明示 opt-in のときだけ章ファイルの filename prefix を整える

非責務:

- `manga/pages/` の追加、削除、並び替え
- manga の chapter / episode metadata 管理
- filename prefix を正として章順を決めること
- 既存章ファイルの rename や renumber を既定動作にすること
- `renumber` 実行時を除き chapter file path を rename すること
- Markdown 本文中の link destination を自動 rewrite すること

v0.2 の最小要件:

- `project.type != manga` にのみ対応する
- 章順は `manuscript.chapters` の配列順を正とする
- `move` は `book.yml` を更新するだけで、既定では file rename を行わない
- `remove` は既定では config から外すだけとし、物理削除は明示 opt-in に限る
- `renumber` は `manuscript.chapters` の順序を保ったまま chapter file path と対応する `sections.file` を更新する

### 7.9 `shosei page check`

漫画 project のページ順と見開き候補を検査する。

主な責務:

- `manga/pages/` の辞書順をページ順として確認する
- 数値順と辞書順がずれるファイル名を warning として示す
- ページサイズ不一致を検出する
- 見開き候補と `manga.spread_policy_for_kindle` の整合を確認する
- `manga.front_color_pages` と `manga.body_mode` の整合を確認する
- 機械可読レポートを出力する

v0.2 の最小要件:

- `project.type: manga` にのみ対応する
- `dist/reports/<book-id>-page-check.json` を出力する
- page order と spread candidate を text summary でも示せる

### 7.10 `shosei series sync`

`series.yml` を正として巻一覧と派生 metadata を同期する。

v0.2 の最小要件:

- `shared/metadata/series-catalog.yml` を生成する
- `shared/metadata/series-catalog.md` を生成する
- prose book では `shared/metadata/series-catalog.md` を `manuscript.backmatter` に同期する
- 手書き本文 Markdown を直接 rewrite しない

### 7.11 `shosei story scaffold`

repo-native な物語補助 workspace を生成する。

- `shosei story scaffold`
- `shosei story scaffold --book vol-01`
- `shosei story scaffold --shared`

主な責務:

- `single-book` では `story/` を生成する
- `series` では `shared/metadata/story/` と `books/<book-id>/story/` を分ける
- `README.md` と最小 template file を生成する
- 既存 file は既定で保持し、`--force` のときだけ上書きする

非責務:

- story schema 全体の validation
- scene map や continuity lint
- 本文の自動生成
- `book.yml` / `series.yml` への story field 追加

v0.2 の最小要件:

- manual-first の scaffold のみ提供する
- `single-book` / `series` の repo discovery ルールに従う
- shared canon と巻固有 story workspace を混同しない

### 7.12 `shosei story map`

book-scoped な `scenes.yml` を読み、scene 一覧と report を出力する。

- `shosei story map`
- `shosei story map --book vol-01`

主な責務:

- `single-book` の `story/scenes.yml` を読む
- `series` の `books/<book-id>/story/scenes.yml` を読む
- list order をそのまま scene order として扱う
- `single-book` では `dist/reports/default-story-map.json` を、`series` では `dist/reports/<book-id>-story-map.json` を出力する

非責務:

- shared canon の解析
- entity directory の暗黙走査
- story validation 全般
- 本文 file の存在確認や cross-reference 整合確認

v0.2 の最小要件:

- scene entry の最小 shape は `file` と optional `title`
- `file` は repo-relative path として解釈する
- text summary と JSON report を返す

### 7.13 `shosei story check`

book-scoped な `scenes.yml` を読み、軽い整合チェック結果を report として出力する。

- `shosei story check`
- `shosei story check --book vol-01`

主な責務:

- duplicate `file` entry を warning として報告する
- invalid repo-relative path を error として報告する
- repo 内に存在しない scene file を warning として報告する
- book-scoped story entity Markdown を scan して entity ID を集める
- scene frontmatter の entity 参照を検査する
- `single-book` では `dist/reports/default-story-check.json` を、`series` では `dist/reports/<book-id>-story-check.json` を出力する

非責務:

- shared canon drift の検査
- semantic continuity lint
- 本文内容の解析

v0.2 の最小要件:

- `scenes.yml` と book-scoped story entity Markdown を入力にする
- issue 数と report path を summary で返す
- error issue がある場合は non-zero exit で返せる
- `characters`, `locations`, `terms`, `factions` の entity ID は frontmatter `id` を優先し、未指定時は filename stem を使う
- duplicate entity `id` は error とする
- `series` では scene 参照解決時に book-scoped story data と `shared/metadata/story/` の両方を見る
- scene frontmatter の未解決 entity 参照は warning とする
- invalid scene/entity frontmatter は error とする

### 7.14 `shosei story drift`

`series` における shared canon と巻固有 story data の衝突を report として出力する。

- `shosei story drift --book vol-01`

主な責務:

- `shared/metadata/story/` と `books/<book-id>/story/` を比較する
- machine-readable な `drifts` 配列を report に含める
- same-scope duplicate entity `id` を error として報告する
- shared/book で内容が分岐した同一 kind + `id` を drift error として報告する
- shared/book で内容が同じ同一 kind + `id` を redundant copy warning として報告する
- `dist/reports/<book-id>-story-drift.json` を出力する

非責務:

- scene file や `scenes.yml` の検査
- semantic continuity lint
- 本文内容の解析

v0.2 の最小要件:

- `series` のみ対象とする
- report path と issue 数を summary で返す
- error issue がある場合は non-zero exit で返せる

### 7.15 `shosei story sync`

`series` で shared canon と巻固有 story workspace の間を明示コピーする。単体 sync と `story drift` report を使う batch sync の両方を扱う。

- `shosei story sync --book vol-01 --from shared --kind character --id lead`
- `shosei story sync --book vol-01 --to shared --kind character --id lead`
- `shosei story sync --book vol-01 --from shared --kind character --id lead --force`
- `shosei story sync --book vol-01 --to shared --kind character --id lead --force`
- `shosei story sync --book vol-01 --from shared --report dist/reports/vol-01-story-drift.json --force`
- `shosei story sync --book vol-01 --to shared --report dist/reports/vol-01-story-drift.json --force`

主な責務:

- 単体 mode では source scope から `kind` + `id` で 1 entity を選ぶ
- `--from shared` では `books/<book-id>/story/` へ同じ entity を copy する
- `--to shared` では `shared/metadata/story/` へ同じ entity を copy する
- `--report` 時は `story drift` report の `drifts` 配列を読んで対象 entity 群を確定する
- destination 側に diverged copy がある場合、`--force` が無ければ error にする
- `--force` 時のみ source 内容で destination 側を上書きする

非責務:

- `scenes.yml` の更新
- automatic merge

v0.2 の最小要件:

- `series` のみ対象とする
- `--from shared` か `--to shared` のどちらか一方を必須にする
- 単体 mode では `kind` を `character|location|term|faction` から 1 件指定する
- 単体 mode では `id` を 1 件だけ指定する
- report mode では `--report` を必須にし、`--kind` / `--id` は受け付けない
- report mode は `--force` を必須にする

### 7.16 `shosei reference scaffold`

repo-native な参考資料 workspace を生成する。

- `shosei reference scaffold`
- `shosei reference scaffold --book vol-01`
- `shosei reference scaffold --shared`

主な責務:

- `single-book` では `references/` を生成する
- `series` では `shared/metadata/references/` と `books/<book-id>/references/` を分ける
- `README.md` と最小 template file を生成する
- 既存 file は既定で保持し、`--force` のときだけ上書きする

非責務:

- reference entry の検索や一覧
- broken link の検査
- `editorial.claims.yml` や本文との自動照合
- `book.yml` / `series.yml` への reference field 追加

v0.2 の最小要件:

- manual-first の scaffold のみ提供する
- `single-book` / `series` の repo discovery ルールに従う
- shared reference と巻固有 reference を混同しない
- reference entry は Markdown + frontmatter の 1 file 1 entry を前提にする

### 7.17 `shosei reference map`

reference entry 一覧を読み、text と report を出力する。

- `shosei reference map`
- `shosei reference map --book vol-01`
- `shosei reference map --shared`

主な責務:

- `single-book` では `references/entries/` を読む
- `series` では `shared/metadata/references/entries/` または `books/<book-id>/references/entries/` を読む
- entry 数と一覧を text summary に出す
- machine-readable な JSON report を出力する

非責務:

- duplicate `id` の検査
- broken link の検査
- 本文や `editorial.claims.yml` との参照整合
- shared/book 間の同期

v0.2 の最小要件:

- manual-first の一覧出力に留める
- `single-book` / `series` の repo discovery ルールに従う
- shared reference と巻固有 reference を混同しない
- frontmatter に `id`, `title`, `links`, `tags`, `related_sections`, `status` を置ける
- `id` は frontmatter 優先、未指定時は filename stem を使う

### 7.18 `shosei reference check`

reference entry を検査し、issue report を出力する。

- `shosei reference check`
- `shosei reference check --book vol-01`
- `shosei reference check --shared`

主な責務:

- `single-book` では `references/entries/` を読む
- `series` では `shared/metadata/references/entries/` または `books/<book-id>/references/entries/` を読む
- frontmatter shape の破損を error として report する
- duplicate `id` を error として report する
- `links` と `related_sections` の local path を軽く検査し、missing target を warning として report する
- prose book では `editorial.claims.yml` の `sources` にある `ref:<id>` を reference entry id として解決する
- machine-readable な JSON report を出力する

非責務:

- 外部 URL の到達確認
- 本文との参照整合
- shared/book 間の drift 判定や同期

v0.2 の最小要件:

- manual-first の lightweight check に留める
- `single-book` / `series` の repo discovery ルールに従う
- shared reference と巻固有 reference を混同しない
- `links` は URL または repo-relative path を受け付ける
- `related_sections` は repo-relative path を受け付ける
- prose book では `editorial.claims.yml` の `ref:<id>` source を同じ book の reference id と照合できる

### 7.19 `shosei reference drift`

`series` で shared reference と巻固有 reference の重複、分岐、gap を report 化する。

- `shosei reference drift --book vol-01`

主な責務:

- `shared/metadata/references/entries/` と `books/<book-id>/references/entries/` を比較する
- 同じ `id` を shared と book の両方が持つ entry を `drifts` 配列に出す
- 同一内容なら warning として `redundant-copy` を report する
- 異なる内容なら error として `drift` を report する
- shared にだけある entry と book にだけある entry を `gaps` 配列に出す
- `shared-only`, `book-only` gap は warning として report する
- invalid frontmatter や same-scope duplicate `id` も issue として report する
- machine-readable な JSON report を出力する

非責務:

- shared/book 間の自動同期
- local path や外部 URL の再検証

v0.2 の最小要件:

- `series` の book scope 専用にする
- repo root からは `--book <book-id>` を要求する
- `entries/` directory が存在しない scope は empty として扱う
- `id` は frontmatter 優先、未指定時は filename stem を使う

### 7.20 `shosei reference sync`

`series` で shared reference と巻固有 reference の間を明示コピーする。単体 sync と `reference drift` report を使う batch sync の両方を扱う。

- `shosei reference sync --book vol-01 --from shared --id market`
- `shosei reference sync --book vol-01 --to shared --id market`
- `shosei reference sync --book vol-01 --from shared --id market --force`
- `shosei reference sync --book vol-01 --to shared --id market --force`
- `shosei reference sync --book vol-01 --from shared --report dist/reports/vol-01-reference-drift.json --force`
- `shosei reference sync --book vol-01 --to shared --report dist/reports/vol-01-reference-drift.json --force`

主な責務:

- 単体 mode では source scope から `id` で 1 entry を選ぶ
- `--from shared` では `books/<book-id>/references/entries/` へ同じ entry を copy する
- `--to shared` では `shared/metadata/references/entries/` へ同じ entry を copy する
- `--report` 時は `reference drift` report の `drifts` 配列と source 側に存在する `gaps` を読んで対象 entry 群を確定する
- `--from shared` は `shared-only` gap を適用し、`book-only` gap は skip する
- `--to shared` は `book-only` gap を適用し、`shared-only` gap は skip する
- destination 側に diverged copy がある場合、`--force` が無ければ error にする
- `--force` 時のみ source 内容で destination 側を上書きする

非責務:

- automatic merge
- local path や外部 URL の再検証

v0.2 の最小要件:

- `series` のみ対象とする
- `--from shared` か `--to shared` のどちらか一方を必須にする
- 単体 mode では `id` を 1 件だけ指定する
- report mode では `--report` を必須にし、`--id` は受け付けない
- report mode は `--force` を必須にする

### 7.21 将来候補

- `shosei release`: handoff + tag 前提の成果物固定化
- `shosei page add`
- `shosei migrate --to series --book-id <id>`

## 8. 設定ファイル仕様

設定ファイルは `book.yml` を標準とする。

正式な項目定義は [設定ファイル schema](config-schema.md) を参照する。
探索順・継承・優先順位は [設定探索と継承ルール](config-loading.md) を参照する。

```yaml
project:
  type: light-novel
  vcs: git

book:
  title: "作品名"
  authors:
    - "著者名"
  language: ja
  profile: light-novel
  writing_mode: vertical-rl
  reading_direction: rtl

layout:
  binding: right
  chapter_start_page: odd
  allow_blank_pages: true

cover:
  ebook_image: assets/cover/front.jpg

manuscript:
  frontmatter:
    - manuscript/00-title.md
  chapters:
    - manuscript/01-chapter-1.md
  backmatter:
    - manuscript/99-colophon.md

outputs:
  kindle:
    enabled: true
    target: kindle-ja
  print:
    enabled: true
    target: print-jp-pdfx1a

pdf:
  engine: chromium
  toc: true
  page_number: true
  running_header: auto

print:
  trim_size: bunko
  bleed: 3mm
  crop_marks: true
  body_pdf: true
  cover_pdf: false
  pdf_standard: pdfx1a

images:
  default_caption: optional
  default_alt: required
  spread_policy_for_kindle: split
  default_page_side: either
  min_print_dpi: 300

validation:
  strict: true
  epubcheck: true
  accessibility: warn
  missing_image: error
  missing_alt: error
  broken_link: error

git:
  lfs: true
  lockable:
    - "*.psd"
    - "*.clip"
    - "*.kra"
    - "*.tif"
```

## 9. 原稿モデル

### 9.1 prose 系

対象:

- business
- paper
- novel
- light-novel

単位:

- section
- chapter
- figure

ソース:

- Markdown
- 画像アセット
- CSS
- フォント

### 9.2 manga 系

単位:

- volume
- chapter
- page
- spread

ソース:

- script
- storyboard metadata
- page images
- spread metadata

漫画は文章主体の flow と別の build graph を持つ。

## 10. 縦書き・横書き

縦書き・横書きは本全体の見た目設定ではなく、原稿モデルの属性として扱う。

要件:

- 本全体の既定値を持つ
- セクション単位 override を許可する
- `titlepage`, `colophon`, `appendix` などで個別指定可能
- PDF/EPUB どちらでも target profile に応じた表現へ変換する

## 11. 画像仕様

画像は first-class feature とする。

### 11.1 配置モード

- `inline`
- `block`
- `full-width`
- `full-page`
- `spread`
- `chapter-frontispiece`

### 11.2 対象カテゴリ別の考え方

- `business`: 図表・スクリーンショット中心
- `paper`: 図表・表・引用中心
- `novel`: 章扉・挿絵中心
- `light-novel`: 口絵・挿絵・見開き重視
- `manga`: ページ画像と見開きが中心

### 11.3 振る舞い

- 印刷向けでは `spread` を正式対応
- Kindle 向けでは `spread` を best-effort で劣化
- 劣化ポリシー:
  - `split`
  - `single-page`
  - `skip`

### 11.4 将来対応候補

- 左右面指定
- ノド補正
- カラーページ束管理

## 12. Section type

単なるファイル列ではなく、章や付帯要素に意味を持たせる。

対象例:

- `cover`
- `titlepage`
- `toc`
- `chapter`
- `appendix`
- `afterword`
- `colophon`

これにより、Kindle と印刷での出し分けを行いやすくする。

補足:

- `cover.ebook_image` は外部カバー画像を表す
- `sections.type: cover` は本文フローに入るカバーページを表す

### 12.1 構造情報の層

v0.2 では、本文やページの構造情報を次の 3 層に分けて扱う。

- source structure
  - どのファイル、またはどのページ画像が、どの順で流れるか
- semantic structure
  - その要素が `titlepage`, `chapter`, `appendix`, `afterword` などのどれに当たるか
- navigation structure
  - 目次、EPUB nav、PDF bookmark、running header に使う見出し階層

同じ原稿要素が複数層に関わってもよいが、責務は混在させない。

### 12.2 prose の見出し取り扱い

prose 系では `manuscript.frontmatter`, `manuscript.chapters`, `manuscript.backmatter` が source structure を決める。

navigation structure は Markdown 見出しから導出する。

ルール:

- `manuscript.chapters` に含まれる各本文ファイルは、最初の level-1 heading を章題として扱う
- level-2 以降の heading は節・小節候補として扱う
- `sections.type` はファイルの意味分類に使い、見出し文字列の source of truth にはしない
- `book.yml` に章題や節題を重複定義しない

profile ごとの既定:

- `business`
  - chapter と section の両方を navigation に使うことを優先する
- `paper`
  - chapter よりも section 中心の navigation を優先し、図表・引用・文献を含む本文を扱いやすくする
- `conference-preprint`
  - `paper` の派生 profile として扱い、1 枚配布の短い本文を前提に section 中心の navigation を使う
- `novel`
  - chapter 中心の navigation を既定とし、section は任意扱いにしやすくする
- `light-novel`
  - chapter 中心を既定としつつ、`prologue`, `interlude`, `epilogue`, `afterword` などの付帯セクションを navigation に含めやすくする

### 12.3 manga の構造取り扱い

manga 系ではページ順が source structure の primary source となる。

ルール:

- v0.2 では chapter title や section title を必須にしない
- page image 列だけで build/validate が成立することを優先する
- 章扉、各話タイトル、あとがき、おまけページなどは存在してよいが、v0.2 では page image 自体から見出しを抽出しない
- 章や話の metadata は将来拡張として別途定義できるようにする

### 12.4 出力への利用

navigation structure の利用先は次を含む。

- print 向け目次
- EPUB nav
- PDF bookmark
- running header

v0.2 の既定:

- prose の TOC / EPUB nav / PDF bookmark は Markdown 見出し由来の navigation structure を使う
- `pdf.running_header: chapter` は prose 本文の直近 chapter title を参照する
- `pdf.running_header: auto` は profile ごとの既定を使い、必要に応じて chapter title を参照する
- manga の EPUB nav は page sequence を既定とし、見出し metadata が未定義でも build 可能とする

### 12.5 v0.2 の制約

v0.2 では次を未対応とする。

- config 上での `title`, `short_title`, `include_in_toc` の個別 override
- TOC label と running header label の別管理
- 派生した navigation metadata を手元の manuscript へ書き戻す機能

## 13. 出力 profile

### 13.1 prose profile

- `business`
  - 既定: `horizontal-ltr`
  - 図表中心
- `paper`
  - 既定: `horizontal-ltr`
  - 引用、図表、参考文献中心
- `conference-preprint`
  - `project.type = paper` のときだけ選べる prose profile
  - 既定: `horizontal-ltr`
  - A4、2 段組、短い配布物向けの print preset を優先
- `novel`
  - 既定: `vertical-rl`
  - 挿絵少なめ
- `light-novel`
  - 既定: `vertical-rl`
  - 全ページ画像・見開き対応を強める

### 13.2 target profile

- `kindle-ja`
  - reflowable EPUB を標準
  - `reading_direction` 必須
- `print-jp-pdfx1a`
  - 印刷会社向け保守的設定
- `print-jp-pdfx4`
  - 透明・カラー運用を考慮
- `kindle-comic`
  - 漫画向け固定レイアウト成果物
- `print-manga`
  - 漫画印刷向け本文 PDF

## 14. build パイプライン

### 14.1 prose

1. config 読み込み
2. profile 解決
3. 章順確定
4. アセット解決
5. stylesheet 解決
6. Pandoc 実行
7. target 別後処理
8. 検証
9. handoff 成果物作成

v0.2 の既定:

- prose Kindle / EPUB build では `base.css` と `epub.css` を Pandoc に渡す
- prose print build では `base.css`, `print.css`, generated layout stylesheet を解決する
  - `pdf.engine = weasyprint` の場合は Pandoc にそのまま渡して PDF を生成する
  - `pdf.engine = chromium` の場合は Pandoc で self-contained HTML を生成し、その HTML を headless Chromium で PDF 化する
  - `novel`, `light-novel` の tighter print typography は `print.css` 側の責務とし、扉 / 目次と本文のページ分離は generated layout stylesheet 側の責務とする
- `series` repo では prose stylesheet は `shared/styles/` から解決する

### 14.2 manga

1. config 読み込み
2. ページマニフェスト解決
3. 見開き/面付け解決
4. 画像検証
5. target 別成果物生成
6. 検証
7. handoff 成果物作成

v0.2 の既定:

- 明示 page manifest schema が未定義の間、`manga/pages/` 直下の PNG / JPEG を辞書順で解決してページ順とみなす
- `manga/pages/` が無い、または対象画像が 1 枚も無い場合は preflight error
- 明示 spread metadata が未定義の間、Kindle 向けの `spread_policy_for_kindle` は横長ページを見開き候補として扱う
- `split` は横長ページを 2 分割し、`book.reading_direction` に従って Kindle ページ順へ並べる
- `single-page` は横長ページを 1 ページのまま残す
- `skip` は横長ページを Kindle 出力から除外し、結果が 0 ページになる場合は error

## 15. 検証仕様

### 15.1 共通

`validate` は target ごとの preflight report を生成する。

- 欠落ファイル
- リンク切れ
- metadata 不足
- 章順不正
- 画像参照不整合
- prose 原稿の文字数集計
  - YAML frontmatter は除外する
  - Markdown 記法は除いた plain text を数える
  - total / frontmatter / chapters / backmatter と file ごとの内訳を report に含める
- target/profile validation summary
  - Kindle / print の channel, target, book profile を report に含める
  - target / PDF standard / profile default の判定を `ok` または `warning` として report に含める
  - warning の詳細は従来通り `issues[]` にも含める

### 15.2 EPUB

- `epubcheck`
- nav/package metadata
- alt
- language
- heading hierarchy
- accessibility metadata

### 15.3 Kindle

- reading direction
- `cover.ebook_image` と出力 metadata の整合
- reflow を壊す要素の警告
- `validation.kindle_previewer: true` のときは、生成した Kindle artifact に対して Kindle Previewer の conversion check を試みる
- Kindle Previewer が未導入の場合は `validators[]` に `missing-tool` を記録し、それだけでは validate を fail にしない
- Kindle Previewer の conversion check 失敗は validation error として返す
- Kindle Previewer の詳細ログは `dist/logs/<book-id>-kindle-previewer-validate.log` に保存し、report から参照する
- CI では proprietary binary を要求せず、fake executable による report contract の smoke を行う

### 15.4 印刷

- trim size
- bleed
- crop marks
- page margins
- column count / column gap
- simplex / duplex
- page limit
- font embed
- PDF standard
- 生成した PDF への `qpdf --check`

追加ルール:

- print 出力が有効な場合は、生成した PDF に対する `qpdf --check` を validator として実行してよい
- `qpdf` が未導入の場合は `validators[]` に `missing-tool` を記録し、それだけでは validate を fail にしない
- `qpdf` の検査失敗は validation error として返す
- 画像解像度

### 15.5 漫画

- ページ順
- 見開き対応
- 左右ページ整合
- サイズ不一致
- カラーページ整合
- guided view / panel metadata の整合
- Kindle 向け見開き劣化ポリシーの適用結果
- `manga.front_color_pages` が resolved page count を超える場合は error
- `manga.front_color_pages` で指定した巻頭ページが color と判定されない場合は warning
- `manga.body_mode: monochrome` で本文ページが color と判定された場合は error
- `manga.body_mode: color` で本文ページが color と判定されない場合は warning

### 15.6 preflight report

出力方針:

- 端末向け summary
- JSON レポート
- 必要に応じて外部 validator の詳細成果物への参照
- `--json` と file 出力は同じ report schema を共有する
- CLI の text 出力では、JSON レポート順の先頭最大 5 件を `原因 / 発生箇所 / 修正例` の形で続けて表示できる
- issue が 6 件以上ある場合、text 出力では末尾に残件数だけを `... and N more` で示す
- prose の summary では total character count を併記できる

各 issue は次を持つ。

- severity
- target
- 発生箇所
  - file path
  - 特定できる場合は line 番号
- 原因
- 修正例

report には `validators[]` も含める。

各 validator entry は次を持つ。

- name
- target
- status
  - `passed`
  - `warned`
  - `failed`
  - `missing-tool`
  - `skipped`
- artifact path
- log path
- summary

## 16. Git / バージョン管理

本ツールは Git 前提とする。

### 必須要件

- `git init` 補助
- `.gitignore` 自動生成
- `.gitattributes` 自動生成
- build 成果物の commit 情報記録

### 推奨要件

- Git LFS 対応
- lockable asset 設定
- handoff 前の dirty worktree 警告
- build provenance の記録

## 17. 実装・配布要件

### 17.1 実装言語

- 本体実装は Rust とする
- 将来的なライブラリ分離を考慮し、CLI と core を分けやすい構造にする

### 17.2 対応 OS

- macOS
- Windows
- Linux

### 17.3 配布形態

- OS ごとの単一バイナリ配布を基本とする
- package manager 対応は将来追加してもよいが、v0.2 の必須条件ではない

### 17.4 クロスプラットフォーム要件

- 設定ファイルの相対パスは OS 非依存の表現で扱う
- ログ・エラー表示・JSON 出力は OS 間で互換であること
- シェル依存の機能を CLI の必須要件にしない
- 一時ファイル、キャッシュ、dist 配下の扱いを OS 間で揃える

### 17.5 テスト方針

- 少なくとも macOS / Windows / Linux の 3 環境で CLI のスモークテストを持つ
- `init`, `build`, `validate`, `doctor` は 3 OS で共通に検証する
- CI job / step 名だけで、どの OS でどの command-level smoke が通ったか読めること
- 外部依存がない範囲のロジックはユニットテストで閉じる

## 18. 優しいインタフェース要件

- 通常操作は `init`, `build`, `validate`, `preview`, `doctor`, `handoff` に絞る
- raw な外部ツールエラーをそのまま主表示しない
- エラーは `原因 / 発生箇所 / 修正例` の三点で示す
- `発生箇所` は file path を基本とし、特定できる場合は line 番号も含める
- 失敗時の詳細ログは `dist/logs/` に保存する
- 実行結果は人間向け表示と JSON を両方出力できる
- エラーメッセージは OS 固有表現に寄り過ぎず、同じ構造で理解できること

## 19. 納品仕様

### 19.1 `handoff kindle`

- EPUB
- build summary
- build stages
- build inputs
- target profile
- commit hash
- `reports/validate.json`
- 設定済みなら cover asset のコピー
- `dist/handoff/<book-id>-kindle/` に成果物コピーと `manifest.json`
- `manifest.json` には `selected_artifact_details[{channel,target,path,primary_tool,target_profile,artifact_metadata}]` を含める
- `artifact_metadata` には少なくとも `reading_direction`, `fixed_layout`, `cover_ebook_image` を入れられる

### 19.2 `handoff print`

- 本文 PDF
- 必要に応じて表紙 PDF
- 仕様 summary
  - 判型
  - 面指定
  - ページ数
  - PDF standard
  - bleed
  - crop marks
  - fonts embedded
- build stages
- build inputs
- `reports/validate.json`
- commit hash
- `dist/handoff/<book-id>-print/` に成果物コピーと `manifest.json`
- `manifest.json` には `selected_artifact_details[{channel,target,path,primary_tool,target_profile,artifact_metadata}]` を含める
- `artifact_metadata` には少なくとも `trim_size`, `bleed`, `sides`, `pdf_standard`, `page_count`, `fonts_embedded` を入れられる

### 19.3 `handoff proof`

- 校正用 PDF または preview 成果物
- 参照用 EPUB
- validation / preflight summary
- タイトル、巻、target、build 時刻、commit hash を含む manifest
- `manifest.json` には `review_notes`、`review_packet`、`editorial_summary.claim_count`、`editorial_summary.figure_count` も含める
- 外部校正・編集者が参照すべき注意点一覧
- `reports/review-packet.json` に unresolved issue、reviewer note、claim / figure / freshness の構造化一覧
- `reports/review-packet.json` には `issue_summary`、`reviewer_notes`、`editorial_summary` を含める
- editorial sidecar のコピー
- claim / figure / freshness の reviewer note 要約
- v0.2 では `dist/handoff/<book-id>-proof/` に成果物コピー、`manifest.json`、`review-notes.md`、`reports/review-packet.json`、`editorial/` 配下の sidecar コピーを出す
- `manifest.json` には `build_stages`, `build_inputs`, `selected_artifact_details`, `validation_report`, `git_dirty`, `dirty_worktree_warning` も含める
- `selected_artifact_details[*].artifact_metadata` には、proof で同梱した artifact の target/profile 条件や、manga の場合は `source_page_count`, `rendered_page_count`, `spread_policy_for_kindle`, `unique_page_dimensions` を含めてよい

## 20. MVP の範囲

### 必須

- `init`
- `build`
- `validate`
- `doctor`
- prose 系 Pandoc build
- Kindle/印刷向け profile
- 縦書き/横書き指定
- 画像差し込みの基本モード
- Git 前提の初期構成
- Rust 実装
- macOS / Windows / Linux での基本動作

### 推奨

- `explain`
- `preview`
- `preview --watch`
- `validate` の preflight report
- `handoff`
- `handoff proof`
- `series sync`
- `page check`
- manga のページマニフェスト
- Git LFS 案内
- 3 OS CI

### 将来

- fixed-layout EPUB の詳細制御
- Kindle Previewer の深い統合
- 漫画の guided view/panel metadata
- print cover source schema の詳細化
- 印刷会社別 preset

## 21. 外部制約メモ

仕様整理の前提として、以下の制約を考慮している。

- Kindle 日本語向けは EPUB と reading direction の扱いが重要
- 印刷会社向けは PDF/X profile が重要
- EPUB では Accessibility metadata と validation が重要
- 漫画向けは fixed-layout とページ画像管理が重要

詳細な理由は ADR を参照すること。
