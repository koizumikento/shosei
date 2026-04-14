# shosei VS Code extension

`editors/vscode/` は、`shosei` CLI の薄い VS Code adapter です。

方針:

- 出版ロジックは `shosei` CLI に委譲する
- VS Code 側では command 実行、output channel、Problems 反映を担当する
- `validate` と `page check` は既存 JSON report を再利用する
- 専用 sidebar view から repo 状態と主要操作にアクセスする

## Commands

- `Shosei: Init`
- `Shosei: Chapter Add`
- `Shosei: Chapter Move`
- `Shosei: Chapter Remove`
- `Shosei: Chapter Renumber`
- `Shosei: Explain`
- `Shosei: Validate`
- `Shosei: Build`
- `Shosei: Preview`
- `Shosei: Preview (Watch)`
- `Shosei: Reference Scaffold`
- `Shosei: Reference Map`
- `Shosei: Reference Check`
- `Shosei: Reference Drift`
- `Shosei: Reference Sync`
- `Shosei: Doctor`
- `Shosei: Page Check`
- `Shosei: Series Sync`
- `Shosei: Select Book`
- `Shosei: Refresh View`

## View

Activity Bar に `Shosei` view container を追加する。

- `Context`: repo mode、repo root、series の target book
- `Toolchain`: host OS、required / optional summary、tool status
- `Resolved Config`: title、project type、language、outputs、writing mode、binding、editorial summary
- `Structure`: config file、chapter list、editorial sidecar file
- `Actions`: explain / validate / build / preview / doctor / reference などの主要操作

repo が見つからない場合は、view から `Init` を直接起動できる。

`series` では view から target book を選べる。選択値は workspace state に保持し、コマンド実行時の `--book` に使う。

prose project では chapter item の context menu から move / remove を呼べる。add / renumber は action と command palette から使う。

reference surface は command palette と sidebar action から使う。`reference scaffold|map|check` は single-book / series shared / series book を切り替えて起動でき、`reference drift|sync` は series book を対象に実行する。`reference map` / `reference check` は対象 workspace が未初期化なら `reference scaffold` を提案する。`reference check` と `reference drift` は CLI report を読んで Problems に反映する。

## Settings

既定ではインストール済みの `shosei` を実行する。

```json
{
  "shosei.cli.command": "shosei",
  "shosei.cli.args": []
}
```

この source tree の CLI を直接使う場合は、`cwd` が対象 book repo になっても動くように `--manifest-path` を付ける。

```json
{
  "shosei.cli.command": "cargo",
  "shosei.cli.args": [
    "run",
    "--manifest-path",
    "/path/to/cb-tools/crates/shosei-cli/Cargo.toml",
    "--bin",
    "shosei",
    "--"
  ]
}
```

`series` repo で active file が `books/<book-id>/` 配下にない場合は、`shosei.series.defaultBookId` を設定できる。

## Development

VS Code で repo root を開けば、`.vscode/launch.json` の `shosei: Extension Development Host` からそのまま `F5` で拡張を起動できる。開発ホストは `--disable-extensions` 付きで立ち上げ、手元の他拡張の activation error を切り離す。

開発ホストでは `shosei.cli.command` / `shosei.cli.args` が未設定でも、repo 内の `crates/shosei-cli/Cargo.toml` が見つかれば `cargo run --manifest-path ... --bin shosei --` に自動フォールバックする。

`Shosei: Init` は VS Code 側で template / `paper` の場合は profile / repo mode / title / author / language / output preset を集め、`shosei init <path> --non-interactive ...` に変換して実行する。`paper` は print を先頭に出し、`conference-preprint` は `--config-profile conference-preprint` に変換する。scaffold 自体は CLI が生成する。

view の config / structure 表示は `shosei explain --json`、toolchain 表示は `shosei doctor --json` を使っており、required / optional の分類も含めて VS Code 側で config merge や依存検出を再実装しない。

Extension Development Host 側でこの source tree の CLI を使うときは、workspace settings を次のようにする。

```json
{
  "shosei.cli.command": "cargo",
  "shosei.cli.args": [
    "run",
    "--manifest-path",
    "/path/to/cb-tools/crates/shosei-cli/Cargo.toml",
    "--bin",
    "shosei",
    "--"
  ]
}
```

構文チェック:

```bash
node --check extension.js
node --check src/core.js
node --check src/view.js
```

テスト:

```bash
node --test ./test/**/*.test.js
```
