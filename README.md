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
- prose 側では `paper` と `conference-preprint` のような論文系 profile も扱う

対象とする主な出力:

- `kindle-ja`
- `print-jp-pdfx1a`
- `print-jp-pdfx4`

## 現在の状態

`v0.1` の現状は、完成版の制作ツールではなく、CLI surface とコア設計の立ち上げ段階です。

今あるもの:

- `shosei init` による基本対話つき scaffold 生成
- `book.yml` / `series.yml` を前提にしたリポジトリ探索
- `shosei explain` による解決済み設定と値の由来の表示
- prose / manga project に対する build / validate / handoff の基本導線
- `shosei preview` の one-shot / `--watch` 生成導線
- `shosei chapter add|move|remove` による prose の章順管理
- `shosei series sync` による series metadata と prose backmatter の同期
- `shosei page check` による manga ページ順・見開き候補・カラーポリシー確認
- `shosei doctor` による依存解決結果と導入ヒントの表示
- `editors/vscode/` による CLI 実処理委譲型の VS Code extension scaffold
- config error / preflight error を潰さず返すテスト

まだ未実装のもの:

- target/profile ごとの検証強化の残り
- `init` の完全な分岐質問つきウィザード

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
mkdir my-book
cd my-book
shosei init
shosei explain
shosei build
shosei validate
```

`init` は現在、短い対話式です。既定値で一気に scaffold を作る場合は `--non-interactive --config-template <template>` を使えます。`--title`, `--author`, `--language`, `--output-preset`, `--repo-mode` で対話項目を明示 override できます。利用できるテンプレートは `business`, `paper`, `novel`, `light-novel`, `manga` です。`paper` では `--config-profile paper|conference-preprint` を受け付けます。

```bash
shosei init ./preprint --config-template paper --config-profile conference-preprint --non-interactive
```

### series

漫画テンプレートは既定で `series` 構成を生成します。

```bash
shosei init ./my-series --non-interactive --config-template manga --title "My Series"
cd my-series
shosei explain --book vol-01
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
  .agents/
    skills/
      shosei-project/
        SKILL.md
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
  .agents/
    skills/
      shosei-project/
        SKILL.md
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
| `shosei handoff <destination>` | handoff package を生成する | 利用可能 |

## 生成される初期構成

`init` はテンプレートに応じて、以下のような土台を生成します。

- `book.yml` または `series.yml`
- `manuscript/` または `manga/`
- `assets/cover/`, `assets/images/`, `assets/fonts/`
- `styles/`
- `dist/`
- `.gitignore`
- `.gitattributes`
- `.agents/skills/shosei-project/SKILL.md`

prose 系テンプレートでは、最初の原稿ファイルも生成します。`paper` / `conference-preprint` は `manuscript/01-main.md`、その他の prose は `manuscript/01-chapter-1.md` です。

この `01-` prefix は初期命名の慣例です。prose の章順は filename prefix ではなく `book.yml` の `manuscript.chapters` で決まります。prefix を整えたい場合は `shosei chapter renumber` を明示的に使います。

参考リンクや作業メモを repo 内で管理したい場合は、初期 scaffold の後で `shosei reference scaffold` を明示実行して `references/` を生成します。`series` では共有用の `shared/metadata/references/` と巻固有の `books/<book-id>/references/` を分けます。現在の `reference` surface は `shosei reference map` で entry 一覧を確認でき、`shosei reference check` で frontmatter / duplicate `id` / local path の軽い整合チェックに加えて prose book の `editorial.claims.yml` にある `ref:<id>` source も照合でき、`shosei reference drift --book <book-id>` で shared/book 間の衝突と片側 gap を確認でき、`shosei reference sync --book <book-id> --from shared|--to shared ...` で明示反映できます。

物語補助を使いたい場合は、初期 scaffold の後で `shosei story scaffold` を明示実行して `story/` または series 用の story workspace を生成します。scene 一覧は `shosei story map`、軽い整合チェックは `shosei story check` で `scenes.yml` と scene/entity frontmatter から report 化できます。series では book-scoped story data に加えて `shared/metadata/story/` の canon も参照解決に使い、scope 間の衝突は `shosei story drift --book <book-id>` で確認できます。scope 間で 1 entity を明示同期するときは `shosei story sync --book <book-id> --from shared --kind <kind> --id <id>` または `shosei story sync --book <book-id> --to shared --kind <kind> --id <id>` を使い、`story drift` report の `drifts` 配列をまとめて反映したいときは `shosei story sync --book <book-id> --from shared --report dist/reports/<book-id>-story-drift.json --force` のように batch 適用できます。

`shosei series sync` は `series.yml` を正として `shared/metadata/series-catalog.yml` と `shared/metadata/series-catalog.md` を生成します。prose book では `shared/metadata/series-catalog.md` を `manuscript.backmatter` に同期します。

## ドキュメント

仕様と ADR は `docs/` にあります。

- [docs/README.md](docs/README.md)
- [機能仕様](docs/specs/functional-spec.md)
- [リポジトリ管理モデル](docs/specs/repository-model.md)
- [設定 schema](docs/specs/config-schema.md)
- [init ウィザード仕様](docs/specs/init-wizard.md)
- [参考資料ワークスペース仕様](docs/specs/reference-workspace.md)
- [Rust 実装アーキテクチャ](docs/specs/rust-architecture.md)
- [物語補助仕様](docs/specs/story-support.md)
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
