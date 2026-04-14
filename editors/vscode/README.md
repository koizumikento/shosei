# shosei VS Code extension

`editors/vscode/` は、`shosei` CLI の薄い VS Code adapter です。

方針:

- 出版ロジックは `shosei` CLI に委譲する
- VS Code 側では command 実行、output channel、Problems 反映を担当する
- `validate` と `page check` は既存 JSON report を再利用する
- 専用 sidebar view から repo 状態と主要操作にアクセスする

## Commands

- `Shosei: Explain`
- `Shosei: Validate`
- `Shosei: Build`
- `Shosei: Preview`
- `Shosei: Preview (Watch)`
- `Shosei: Doctor`
- `Shosei: Page Check`
- `Shosei: Series Sync`
- `Shosei: Select Book`
- `Shosei: Refresh View`

## View

Activity Bar に `Shosei` view container を追加する。

- `Context`: repo mode、repo root、series の target book
- `Actions`: explain / validate / build / preview / doctor などの主要操作

`series` では view から target book を選べる。選択値は workspace state に保持し、コマンド実行時の `--book` に使う。

## Settings

既定ではインストール済みの `shosei` を実行する。

```json
{
  "shosei.cli.command": "shosei",
  "shosei.cli.args": []
}
```

この source tree の CLI を直接使う場合:

```json
{
  "shosei.cli.command": "cargo",
  "shosei.cli.args": ["run", "-p", "shosei-cli", "--bin", "shosei", "--"]
}
```

`series` repo で active file が `books/<book-id>/` 配下にない場合は、`shosei.series.defaultBookId` を設定できる。

## Development

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
