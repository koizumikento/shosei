# shosei

`shosei` は、日本語の出版ワークフローを扱う Rust 製 CLI です。  
EPUB / PDF / Kindle / 印刷入稿を視野に入れつつ、文章書籍と漫画の両方を同じリポジトリモデルで扱うことを目指しています。

現在のリポジトリは spec-first で進んでおり、仕様の整理と Rust 実装の土台づくりが中心です。

## 何を目指すツールか

- 日本語書籍向けの制作フローを 1 つの CLI に集約する
- `book.yml` / `series.yml` を中心に、設定ファイル駆動で扱う
- `single-book` と `series` の 2 つのリポジトリ管理モデルを正式に扱う
- macOS / Windows / Linux で同じコマンド体系を維持する
- prose と manga で入力モデルを分けつつ、共通の運用で扱えるようにする

対象とする主な出力:

- `kindle-ja`
- `print-jp-pdfx1a`
- `print-jp-pdfx4`

## 現在の状態

`v0.1` の現状は、完成版の制作ツールではなく、CLI surface とコア設計の立ち上げ段階です。

今あるもの:

- `shosei init` による初期 scaffold 生成
- `book.yml` / `series.yml` を前提にしたリポジトリ探索
- prose 系 project に対する config 読み込みと build / validate の事前計画
- config error / preflight error を潰さず返すテスト

まだ未実装のもの:

- 実際の EPUB / PDF 生成
- `preview` の本体
- `doctor` の依存検査
- `handoff` の成果物パッケージング
- manga build pipeline の本体

README のコマンド例は、今の CLI surface に合わせています。`cargo run -p shosei-cli --bin shosei -- --help` で確認済みです。

## インストール

現時点では source install 前提です。

```bash
git clone <your-fork-or-repo-url>
cd cb-tools
cargo install --path crates/shosei-cli
```

インストールせずに試すだけなら:

```bash
cargo run -p shosei-cli --bin shosei -- --help
```

## クイックスタート

### single-book

```bash
shosei init ./my-book --config-template novel
cd my-book
shosei build
shosei validate
```

`init` は現在、完全な対話ウィザードではなく、テンプレートごとの既定値で scaffold を生成します。  
利用できるテンプレートは `business`, `novel`, `light-novel`, `manga` です。

### series

漫画テンプレートは既定で `series` 構成を生成します。

```bash
shosei init ./my-series --config-template manga
cd my-series
shosei build --book vol-01
shosei validate --book vol-01
```

`series` repo では、次のどちらかが必要です。

- repo root から `--book <book-id>` を付けて実行する
- `books/<book-id>/...` の内側に移動して実行する

## リポジトリモデル

`shosei` が正式に扱う管理モデルは 2 つです。

### `single-book`

1 冊、または 1 巻を 1 リポジトリとして管理します。

```text
repo/
  book.yml
  manuscript/
  manga/
  assets/
  styles/
  dist/
```

### `series`

シリーズ全体を 1 リポジトリにまとめ、各巻を `books/<book-id>/` 配下に持ちます。

```text
repo/
  series.yml
  shared/
  books/
    vol-01/
      book.yml
      manuscript/
      manga/
      assets/
  dist/
```

設定ファイル名は次で固定です。

- `single-book`: `book.yml`
- `series`: `series.yml` + `books/<book-id>/book.yml`

詳細は [docs/specs/repository-model.md](docs/specs/repository-model.md) を参照してください。

## コマンド

現在の CLI surface:

| Command | Purpose | Status |
|---|---|---|
| `shosei init` | project scaffold を作る | 利用可能 |
| `shosei build` | build plan を解決する | prose 系の planning が中心 |
| `shosei validate` | config / preflight を検証する | 利用可能 |
| `shosei preview` | preview 導線 | プレースホルダー |
| `shosei doctor` | 依存チェック導線 | プレースホルダー |
| `shosei handoff <destination>` | handoff 導線 | プレースホルダー |

## 生成される初期構成

`init` はテンプレートに応じて、以下のような土台を生成します。

- `book.yml` または `series.yml`
- `manuscript/` または `manga/`
- `assets/cover/`, `assets/images/`, `assets/fonts/`
- `styles/`
- `dist/`
- `.gitignore`
- `.gitattributes`

prose 系テンプレートでは、最初の章ファイルとして `manuscript/01-chapter-1.md` も生成します。

## ドキュメント

仕様と ADR は `docs/` にあります。

- [docs/README.md](docs/README.md)
- [機能仕様](docs/specs/functional-spec.md)
- [リポジトリ管理モデル](docs/specs/repository-model.md)
- [設定 schema](docs/specs/config-schema.md)
- [init ウィザード仕様](docs/specs/init-wizard.md)
- [Rust 実装アーキテクチャ](docs/specs/rust-architecture.md)
- [ADR 一覧](docs/adr/README.md)

## 開発

フォーマット:

```bash
cargo fmt
```

Lint:

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

テスト:

```bash
cargo test --workspace
```

repo discovery の focused check:

```bash
cargo test -p shosei-core --test repo_discovery
```

CLI smoke check:

```bash
cargo run -p shosei-cli --bin shosei -- --help
```

## コントリビュート

Issue / PR は歓迎です。  
このリポジトリは spec-first なので、挙動変更を含む提案では実装だけでなく `docs/specs/` と `docs/adr/` の更新も合わせて検討してください。

CLI 名は `shosei` です。設定ファイル名は `book.yml` と `series.yml` を維持します。

## ライセンス

Cargo workspace の license metadata は `MIT` です。
