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
- `Shosei: Story Scaffold`
- `Shosei: Story Map`
- `Shosei: Reveal Scene In Index`
- `Shosei: Story Check`
- `Shosei: Story Drift`
- `Shosei: Story Sync`
- `Shosei: Doctor`
- `Shosei: Page Check`
- `Shosei: Series Sync`
- `Shosei: Select Book`
- `Shosei: Refresh View`

## View

Activity Bar に `Shosei` view container を追加する。

- `Context`: repo mode、repo root、series の target book
- `Structure`: config file、chapter list、初期化済み reference workspace file、初期化済み story workspace file、book-scoped な story structure template file、editorial sidecar file
- `Actions`: explain / validate / build / preview / doctor / reference / story などの主要操作
- `Resolved Config`: title、project type、language、outputs、writing mode、binding、editorial summary
- `Toolchain`: host OS、required / optional summary、tool status

repo が見つからない場合は、view から `Init` を直接起動できる。

`series` では view から target book を選べる。選択値は workspace state に保持し、コマンド実行時の `--book` に使う。

prose project では chapter item の context menu から move / remove を呼べる。add / renumber は action と command palette から使う。

reference surface は command palette と sidebar action から使う。`reference scaffold|map|check` は single-book / series shared / series book を切り替えて起動でき、`reference drift|sync` は series book を対象に実行する。`reference map` / `reference check` は対象 workspace が未初期化なら `reference scaffold` を提案する。`reference check` と `reference drift` は CLI report を読んで Problems に反映する。

初期化済みの reference workspace がある場合、`Structure` には single-book の `references/`、または series の current book / shared scope の reference file を出す。

story surface も command palette と sidebar action から使う。`story scaffold` は single-book / series shared / series book を切り替えて起動でき、`story seed` は book-scoped structure template を選んで `scenes.yml` と scene note 下書きを起こす。`story map|check` は current book scope を対象に実行する。`story drift|sync` は series book を対象に実行する。`story map` / `story check` は対象 workspace が未初期化なら `story scaffold` を提案する。`story check` と `story drift` は CLI report を読んで Problems に反映する。scene note item の context action からは、対応する `scenes.yml` entry を直接開ける。

初期化済みの story workspace がある場合、`Structure` には single-book の `story/`、または series の current book / shared scope の story file を出す。book scope では `scene-notes/` の scene note と `structures/` 配下の構成テンプレートも同じ tree に出す。

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
    "/path/to/shosei/crates/shosei-cli/Cargo.toml",
    "--bin",
    "shosei",
    "--"
  ]
}
```

`series` repo で active file が `books/<book-id>/` 配下にない場合は、`shosei.series.defaultBookId` を設定できる。

## Local install

Marketplace に公開しなくても、手元の VS Code にだけ拡張を入れられる。

VSIX を作る:

```bash
cd editors/vscode
npm run package
```

生成物:

```text
editors/vscode/shosei-vscode-0.0.1.vsix
```

インストール方法:

- VS Code で `Extensions: Install from VSIX...` を実行して上の `.vsix` を選ぶ
- `code` CLI が使える場合は `code --install-extension editors/vscode/shosei-vscode-0.0.1.vsix`

ローカル install 後も、実処理は `shosei` CLI に委譲する。source tree の CLI を使いたい場合は、下の `shosei.cli.command` / `shosei.cli.args` 設定を使う。

## GitHub Release

GitHub Release に VSIX を載せる workflow は `.github/workflows/release.yml` にある。

- repo release tag は `shosei-cli` の version に合わせた `v<cli-version>` を使う
- その tag を push すると VSIX と CLI binary archive を package して同名 release に asset として添付する
- `workflow_dispatch` でも実行でき、tag 未指定なら `v<shosei-cli-version>` を使う
- VSIX asset 名は `editors/vscode/package.json` の version を使う

例:

```bash
git tag v0.1.0
git push origin v0.1.0
```

release に載る asset:

```text
shosei-vscode-0.0.1.vsix
shosei-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
shosei-v0.1.0-x86_64-apple-darwin.tar.gz
shosei-v0.1.0-x86_64-pc-windows-msvc.zip
```

## Development

VS Code で repo root を開けば、`.vscode/launch.json` の `shosei: Extension Development Host` からそのまま `F5` で拡張を起動できる。開発ホストは `--disable-extensions` 付きで立ち上げ、手元の他拡張の activation error を切り離す。

開発ホストでは `shosei.cli.command` / `shosei.cli.args` が未設定でも、repo 内の `crates/shosei-cli/Cargo.toml` が見つかれば `cargo run --manifest-path ... --bin shosei --` に自動フォールバックする。

`Shosei: Init` は VS Code 側で template / `paper` の場合は profile / repo mode / `series` の場合は初期 book id / title / author / language / output preset を集め、`shosei init <path> --non-interactive ...` に変換して実行する。`paper` は print を先頭に出し、`conference-preprint` は `--config-profile conference-preprint` に変換する。`series` の初期 book id は `--initial-book-id` に変換し、既定値は `vol-01`、`/`, `\\`, 空白, `.`, `..` は受け付けない。scaffold 自体は CLI が生成する。

view の config / structure 表示は `shosei explain --json`、toolchain 表示は `shosei doctor --json` を使っており、required / optional の分類も含めて VS Code 側で config merge や依存検出を再実装しない。

Extension Development Host 側でこの source tree の CLI を使うときは、workspace settings を次のようにする。

```json
{
  "shosei.cli.command": "cargo",
  "shosei.cli.args": [
    "run",
    "--manifest-path",
    "/path/to/shosei/crates/shosei-cli/Cargo.toml",
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
