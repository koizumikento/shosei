# Rust 実装アーキテクチャ v0.1

作成日: 2026-04-12  
状態: Draft

## 1. 目的

この文書は、本ツールを Rust で実装する際の責務分離と crate 構成を定義する。

対象:

- Cargo 構成
- CLI 層と core 層の境界
- config 読み込み
- build pipeline
- 外部ツール呼び出し
- cross-platform 対応

## 2. 方針

- v0.1 は過剰な micro-crate 化を避ける
- ただし CLI と core は分離する
- ドメインロジックはライブラリ側に寄せる
- 外部プロセス実行は 1 箇所に集約する
- OS 差異は adapter 層で吸収する

## 3. 推奨 Cargo 構成

```text
repo/
  Cargo.toml
  crates/
    shosei-cli/
    shosei-core/
  editors/
    vscode/
```

補足:

- editor integration は Cargo workspace の外に置いてよい
- VS Code extension host では JavaScript を使ってよい
- ただし editor integration は `shosei` CLI を呼び出す UI adapter に留め、repo discovery、config merge、build / validate planning は Rust 側に残す

### 3.1 `shosei-cli`

役割:

- コマンドライン引数の解釈
- 対話式 `init` の UI
- 標準出力/標準エラーへの表示
- exit code の決定
- `shosei-core` 呼び出し
- editor integration から再利用される実行入口

責務に含めないもの:

- config merge
- repo root 探索の本体
- build planning
- 外部ツールの個別実行ロジック

### 3.2 `shosei-core`

役割:

- domain model
- config 読み込み
- repo mode 判定
- build plan 生成
- validate plan 生成
- preflight report 生成
- 外部ツール adapter
- diagnostics

## 4. `shosei-core` のモジュール構成

`shosei-core` はまず 1 crate とし、内部モジュールで分ける。

```text
shosei-core/src/
  lib.rs
  app/
  cli_api/
  config/
  domain/
  editorial/
  repo/
  pipeline/
  toolchain/
  diagnostics/
  fs/
```

### 4.1 `app/`

ユースケース層。

例:

- `init_project`
- `build_book`
- `validate_book`
- `preview_book`
- `doctor`
- `handoff`
- `explain_config`
- `story_scaffold`
- `sync_series`
- `check_pages`

### 4.2 `config/`

設定読込と merge。

含むもの:

- `book.yml` loader
- `series.yml` loader
- schema validation
- defaults merge
- path normalization

### 4.3 `editorial/`

prose 向け editorial sidecar の読込と検証。

含むもの:

- style guide loader
- claim ledger loader
- figure ledger loader
- freshness ledger loader
- editorial diagnostics

### 4.4 `domain/`

ツールの中核型。

例:

- `ProjectType`
- `RepoMode`
- `BookConfig`
- `SeriesConfig`
- `ResolvedBook`
- `TargetProfile`
- `WritingMode`

### 4.5 `repo/`

repo root 探索と context 判定。

例:

- root discovery
- `single-book` / `series` 判定
- `--book` 解決

### 4.6 `pipeline/`

コマンドごとの plan を組み立てる。

例:

- prose build plan
- manga build plan
- validation plan
- handoff plan

### 4.7 `toolchain/`

外部コマンドの adapter。

対象例:

- `git`
- `pandoc`
- `weasyprint`
- `chromium`
- `epubcheck`
- `git-lfs`
- Kindle Previewer

v0.1 の prose print backend は writing mode ごとの正式 support matrix を持つ。

- `horizontal-ltr` prose と `conference-preprint`: `weasyprint`
- `vertical-rl` prose: `chromium`
- `typst`, `lualatex` は将来拡張候補として扱う

### 4.8 `diagnostics/`

エラー、警告、JSON レポート出力用の構造。

### 4.9 `fs/`

ファイル入出力、path 変換、一時ディレクトリ管理。

## 5. `shosei-cli` の構成

```text
shosei-cli/src/
  main.rs
  args.rs
  prompts.rs
  output.rs
  exit_code.rs
```

### `args.rs`

- `clap` ベースの引数定義

### `prompts.rs`

- `shosei init` の対話式入力

### `output.rs`

- text / json の表示

## 6. ドメイン境界

### CLI 層で持つべきもの

- ユーザー入力の受け取り
- プロンプト
- 文字装飾
- 表示形式選択

### core 層で持つべきもの

- どの本を対象にするかの決定
- どの外部ツールを呼ぶかの決定
- 設定解決
- diagnostics 生成

## 7. path モデル

path は 2 種類に分ける。

### 7.1 `RepoPath`

config 上に現れる論理パス。

特徴:

- UTF-8
- `/` 区切り
- repo root 基準の相対パス

### 7.2 `FsPath`

OS 上の実ファイルパス。

特徴:

- `PathBuf` ベース
- Windows の区切り差異を吸収
- 実行時 I/O にのみ使う

方針:

- domain と config は原則 `RepoPath`
- I/O 境界でのみ `FsPath` に変換

## 8. 実行フロー

### `shosei build`

1. CLI で args 取得
2. repo root 探索
3. repo mode 判定
4. target book 解決
5. config load + merge
6. build plan 生成
7. 外部ツール実行
8. diagnostics 出力

### `shosei validate`

1. CLI で args 取得
2. repo root 探索
3. repo mode 判定
4. target book 解決
5. config load + merge
6. validate / preflight plan 生成
7. validator 実行
8. summary と structured report を出力

### `shosei explain`

1. CLI で args 取得
2. repo root 探索
3. repo mode 判定
4. target book 解決
5. config load + merge
6. 値と由来の説明用 view model を生成
7. text summary を出力

### `shosei init`

1. CLI で prompt 実行
2. 入力値を command request に変換
3. core で file plan 生成
4. file writer が実際に出力

## 9. 外部ツール adapter

各ツールごとに `detect`, `version`, `run` を揃える。

例:

```text
trait ToolAdapter {
  fn detect(&self) -> DetectionResult;
  fn version(&self) -> VersionResult;
  fn run(&self, request: RunRequest) -> RunResult;
}
```

方針:

- 実行ファイル名差異は adapter 側で吸収
- 生の stderr は保持するが、そのまま主表示しない
- core では domain 向けの error に変換する

## 10. cross-platform 方針

- shell command string を組み立てない
- `std::process::Command` を使う
- temp dir は OS API に従う
- 改行差異を domain に持ち込まない
- config とログのエンコードは UTF-8 を基本にする

## 11. 推奨依存方針

特定 crate への固定はしないが、次の分類で選ぶ。

- CLI parser
- YAML serde
- diagnostics/reporting
- path utility
- temp file utility

v0.1 の判断:

- まずは安定した定番 crate を選ぶ
- 実装初期から async 化しない
- 同期実行で十分なところは同期のまま保つ

## 12. テスト構成

### `shosei-core`

- unit test
- config merge test
- repo discovery test
- build plan test

### `shosei-cli`

- snapshot test
- smoke test

### CI

- macOS
- Windows
- Linux
- v0.1 の CI gate は 3 OS matrix で次を実行する
  - `cargo fmt --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo test -p shosei-core --test repo_discovery`
  - `cargo run -p shosei-cli --bin shosei -- --help`

最小スモーク対象:

- `shosei validate`
- `shosei page check`
- `shosei handoff proof`
- `shosei --help`

`init`, `build`, `doctor` の command-level smoke は fixture と外部依存の扱いが固まった段階で追加する。

## 13. 将来の分割条件

次の条件が揃ったら `shosei-core` を追加 crate に分ける。

- config が独立再利用される
- toolchain adapter が肥大化する
- prose と manga の pipeline が明確に独立する

そのまでは 2 crate 構成を維持する。
