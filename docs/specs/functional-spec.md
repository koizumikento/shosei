# 電子書籍制作 CLI 機能仕様書 v0.1

作成日: 2026-04-12  
状態: Draft

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
- 納品用成果物の取りまとめ
- Git 前提の履歴管理
- macOS / Windows / Linux で動作する単一 CLI の提供

## 2. スコープ

### 対象カテゴリ

- ビジネス書
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

`business`, `novel`, `light-novel` は Pandoc を中核変換エンジンとして扱う。

- EPUB: Pandoc EPUB3 writer を利用
- PDF: Pandoc + PDF engine を利用
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

## 5. 想定ユーザー

- 技術書・ビジネス書の著者
- 小説・ライトノベルの制作者
- 漫画制作者、同人制作者、小規模出版社
- 入稿担当、組版担当、外注先とのやりとりを行う人

## 6. プロジェクト構成

リポジトリ管理単位の詳細は [リポジトリ管理モデル](repository-model.md) を参照する。

初期構成の標準形は以下とする。

```text
project/
  book.yml
  manuscript/
    00-cover.md
    01-chapter-1.md
  manga/
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
- Git リポジトリ初期化補助
- 依存チェック案内

質問項目:

1. 作品カテゴリ: `business | novel | light-novel | manga`
2. 本文方向: `horizontal-ltr | vertical-rl`
3. 出力先: `kindle | print | both`
4. 判型: `A5 | B6 | bunko | custom`
5. PDF profile: `pdfx1a | pdfx4`
6. タイトル
7. 著者名
8. 言語
9. サンプルファイル生成有無
10. Git 初期化有無

### 7.2 `shosei build`

`book.yml` を読み、有効な target profile の成果物を生成する。

原則:

- 引数なしで実行できる
- 個別指定は例外的に `--only` など最小限に留める
- prose と manga でパイプラインを切り替える

### 7.3 `shosei validate`

原稿・設定・成果物の検証を行う。

`validate` は単なる lint ではなく、提出前の preflight として振る舞う。

原則:

- 有効な出力 target 全体を既定対象にする
- 例外的に `--target kindle|print` で個別実行できる
- 人間向け summary と機械可読レポートを両方出せる

- 共通 lint
- EPUB 検証
- Kindle 想定検証
- 印刷想定検証
- 機械可読レポート出力

### 7.4 `shosei preview`

レイアウト確認用のプレビューを生成または起動する。

実行モード:

- one-shot: 現在状態の preview を生成または起動する
- watch: 原稿・設定・styles・assets の変更を監視し、preview を再生成する

追加要件:

- `watch` は macOS / Windows / Linux で同じコマンド体系を保つ
- shell 固有の file watch 構文に依存しない
- 再生成失敗時も監視プロセスは継続し、差分修正を試しやすくする

確認対象:

- 縦書き/横書き
- 柱・ノンブル
- 章扉
- 画像の回り込み、全ページ、見開き
- 改ページ

### 7.5 `shosei doctor`

外部依存と環境の確認を行う。

対象例:

- `pandoc`
- `epubcheck`
- PDF engine
- Kindle Previewer
- `git`
- `git-lfs`

追加要件:

- macOS / Windows / Linux で実行ファイル名の差異を吸収して検出する
- PATH 上の解決結果とバージョンを表示する
- 不足依存の導入案内を OS 別に出せるようにする

### 7.6 `shosei handoff`

提出先に応じた成果物パッケージを生成する。

- `shosei handoff kindle`
- `shosei handoff print`
- `shosei handoff proof`

内容:

- 本体成果物
- 仕様サマリ
- build 情報
- commit 情報

### 7.7 将来候補

- `shosei explain`: 最終有効設定と値の由来の表示
- `shosei release`: handoff + tag 前提の成果物固定化
- `shosei chapter add|move|remove`
- `shosei page add`
- `shosei page check`: manga のページ順、見開き、panel metadata の確認を含む
- `shosei series sync`: `series.yml` から巻一覧、既刊案内、派生 metadata を同期
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

manuscript:
  frontmatter:
    - manuscript/00-cover.md
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
  engine: weasyprint
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

## 13. 出力 profile

### 13.1 prose profile

- `business`
  - 既定: `horizontal-ltr`
  - 図表中心
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
5. Pandoc 実行
6. target 別後処理
7. 検証
8. handoff 成果物作成

### 14.2 manga

1. config 読み込み
2. ページマニフェスト解決
3. 見開き/面付け解決
4. 画像検証
5. target 別成果物生成
6. 検証
7. handoff 成果物作成

## 15. 検証仕様

### 15.1 共通

`validate` は target ごとの preflight report を生成する。

- 欠落ファイル
- リンク切れ
- metadata 不足
- 章順不正
- 画像参照不整合

### 15.2 EPUB

- `epubcheck`
- nav/package metadata
- alt
- language
- heading hierarchy
- accessibility metadata

### 15.3 Kindle

- reading direction
- cover 整合
- reflow を壊す要素の警告
- 必要に応じて Kindle Previewer 連携
- device preview 由来の警告取り込み

### 15.4 印刷

- trim size
- bleed
- crop marks
- font embed
- PDF standard
- 画像解像度

### 15.5 漫画

- ページ順
- 見開き対応
- 左右ページ整合
- サイズ不一致
- カラーページ整合
- guided view / panel metadata の整合

### 15.6 preflight report

出力方針:

- 端末向け summary
- JSON レポート
- 必要に応じて外部 validator の詳細成果物への参照

各 issue は次を持つ。

- severity
- target
- 発生箇所
- 原因
- 修正例

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
- package manager 対応は将来追加してもよいが、v0.1 の必須条件ではない

### 17.4 クロスプラットフォーム要件

- 設定ファイルの相対パスは OS 非依存の表現で扱う
- ログ・エラー表示・JSON 出力は OS 間で互換であること
- シェル依存の機能を CLI の必須要件にしない
- 一時ファイル、キャッシュ、dist 配下の扱いを OS 間で揃える

### 17.5 テスト方針

- 少なくとも macOS / Windows / Linux の 3 環境で CLI のスモークテストを持つ
- `init`, `build`, `validate`, `doctor` は 3 OS で共通に検証する
- 外部依存がない範囲のロジックはユニットテストで閉じる

## 18. 優しいインタフェース要件

- 通常操作は `init`, `build`, `validate`, `preview`, `doctor`, `handoff` に絞る
- raw な外部ツールエラーをそのまま主表示しない
- エラーは `原因 / 発生箇所 / 修正例` の三点で示す
- 失敗時の詳細ログは `dist/logs/` に保存する
- 実行結果は人間向け表示と JSON を両方出力できる
- エラーメッセージは OS 固有表現に寄り過ぎず、同じ構造で理解できること

## 19. 納品仕様

### 19.1 `handoff kindle`

- EPUB
- build summary
- target profile
- commit hash

### 19.2 `handoff print`

- 本文 PDF
- 必要に応じて表紙 PDF
- 仕様 summary
  - 判型
  - ページ数
  - PDF standard
  - bleed
  - crop marks
  - fonts embedded
- commit hash

### 19.3 `handoff proof`

- 校正用 PDF または preview 成果物
- 参照用 EPUB
- validation / preflight summary
- タイトル、巻、target、build 時刻、commit hash を含む manifest
- 外部校正・編集者が参照すべき注意点一覧

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

- `preview`
- `preview --watch`
- `validate` の preflight report
- `handoff`
- `handoff proof`
- manga のページマニフェスト
- Git LFS 案内
- 3 OS CI

### 将来

- `shosei explain`
- `shosei series sync`
- `shosei page check`
- fixed-layout EPUB の詳細制御
- Kindle Previewer の深い統合
- 漫画の guided view/panel metadata
- cover body 分離の高度化
- 印刷会社別 preset

## 21. 外部制約メモ

仕様整理の前提として、以下の制約を考慮している。

- Kindle 日本語向けは EPUB と reading direction の扱いが重要
- 印刷会社向けは PDF/X profile が重要
- EPUB では Accessibility metadata と validation が重要
- 漫画向けは fixed-layout とページ画像管理が重要

詳細な理由は ADR を参照すること。
