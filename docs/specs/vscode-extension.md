# VS Code 拡張仕様 v0.1

作成日: 2026-04-14  
状態: Draft

## 1. 目的

`shosei` を使う制作リポジトリを VS Code から扱いやすくする。

ただし、拡張は別実装の出版エンジンを持たない。`build` / `validate` / `preview` / `explain` の実処理は既存の `shosei` CLI に委譲し、VS Code 側は editor integration に責務を絞る。

## 2. 設計原則

- `CLI を source of truth にする`
- `repo discovery と config merge を複製しない`
- `実処理は shosei が行い、VS Code 側は command / terminal / diagnostics を仲介する`
- `series` の book 解決は editor context と既存 repo model に合わせる
- `validate` / `page check` の Problems 反映は、既存 JSON report を再利用する

## 3. スコープ

v0.1 の VS Code 拡張は次を扱う。

- `shosei init`
- `shosei explain`
- `shosei validate`
- `shosei build`
- `shosei preview`
- `shosei preview --watch`
- `shosei reference scaffold|map|check|drift|sync`
- `shosei chapter add|move|remove|renumber`
- `shosei doctor`
- `shosei page check`
- `shosei series sync`

設定編集 UI、Pandoc や PDF engine の独自設定画面は v0.1 の対象外とする。

## 4. 実行モデル

### 4.1 CLI 呼び出し

VS Code 拡張は `shosei` を外部プロセスとして実行する。

設定項目:

- `shosei.cli.command`: 既定値 `shosei`
- `shosei.cli.args`: 既定値 `[]`

ローカル開発で source tree の CLI を直接使いたい場合は、対象 repo の `cwd` からでも起動できるよう `--manifest-path` を付けて設定する。

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

### 4.2 guided init

VS Code 側で template / `paper` の場合は profile / repo mode / `series` の場合は初期 book id / title / author / language / output preset を Quick Pick / Input Box で集め、`shosei init <path> --non-interactive ...` に変換して実行してよい。

ルール:

- scaffold 内容は必ず CLI に生成させる
- VS Code 側で `book.yml` / `series.yml` の雛形を書かない
- 成功後に optional で `shosei doctor` を続けて実行してよい
- target path は workspace folder か folder picker から選ばせてよい
- `series` を選んだ場合は `--initial-book-id <book-id>` に変換し、未指定時の既定値は `vol-01` とする
- 初期 book id の validation は CLI と揃え、空文字、`/`, `\\`, 空白, `.`, `..` は受け付けない

### 4.3 one-shot と watch

- one-shot command は child process と output channel で実行する
- `preview --watch` は VS Code task / terminal で実行する
- watch 中の停止は VS Code terminal 側で行う

### 4.4 sidebar summary

専用 view では `shosei explain --json` と `shosei doctor --json` を使い、少なくとも次を表示してよい。

- `Context`: repo mode / repo root / target book
- `Structure`: config file、prose の chapter list、初期化済み reference workspace file、editorial sidecar file
- `Actions`: explain / validate / build / preview / doctor / reference surface
- `Resolved Config`: title / project type / language / outputs / writing mode / binding / editorial summary
- `Toolchain`: host OS / required・optional summary / individual tool status
- prose では chapter add / move / remove / renumber を command palette と chapter item context menu から起動してよい
- reference surface は command palette と sidebar action から起動してよい
- `reference map` / `reference check` の対象 workspace が未初期化なら、拡張は `reference scaffold` 実行を提案してよい

chapter、reference file、sidecar file はクリックで open してよい。

## 5. repo context 解決

### 5.1 repo root

拡張は active file か workspace folder から上方探索し、最も近い `book.yml` または `series.yml` を repo root として扱う。

### 5.2 `single-book`

- `book.yml` を見つけた場合は `single-book` とみなす
- book 指定は不要
- `--path <repo-root>` を付けて CLI を実行する

### 5.3 `series`

`series.yml` を見つけた場合は `series` とみなす。

book 解決の優先順位:

1. 専用ビューで明示的に選んだ book
2. active file または選択中 workspace path が `books/<book-id>/...` 配下にある
3. `shosei.series.defaultBookId`
4. `books/` 配下の候補を Quick Pick で選ぶ
5. 候補列挙ができない場合は Input Box で手入力する

book が必要な command では `--book <book-id> --path <repo-root>` を付ける。

専用ビューで選んだ book は workspace state に保持してよい。

## 6. Diagnostics

### 6.1 `validate`

`shosei validate` は既存の `dist/reports/*-validate.json` を出力する。拡張は CLI 実行後に report path を読み取り、`issues[].location` を VS Code Problems に反映する。

### 6.2 `page check`

`shosei page check` も同様に `dist/reports/*-page-check.json` を読み、manga page まわりの issue を Problems に反映する。

### 6.3 scope

- line/column 情報がない issue は file 単位の diagnostic として先頭行に付与する
- location を持たない issue は output channel に残し、Problems には出さない

### 6.4 `reference check` / `reference drift`

- `shosei reference check` の `dist/reports/*-reference-check.json` を読み、issue を Problems に反映してよい
- `shosei reference drift` の `dist/reports/*-reference-drift.json` も同様に issue を Problems に反映してよい
- `reference map` は text/report 確認用とし、Problems には流さない
- `reference sync` は同期 command として扱い、Problems 連携は必須にしない
- `reference map` / `reference check` 実行前に対象 scope の `entries/` がなければ、Problems 連携より先に scaffold 導線を出してよい

## 7. 実装配置

VS Code 拡張は Cargo workspace とは別に、repo root 配下の次に置く。

```text
repo/
  editors/
    vscode/
      package.json
      extension.js
      src/
      test/
```

補足:

- VS Code extension host 自体は JavaScript で実装してよい
- ただし、出版ロジック、repo model、config loading、pipeline planning は Rust 側から移さない
- v0.1 では sidebar の専用 Tree View を持ってよい

## 7.1 ローカル配布

v0.1 の拡張は Marketplace 公開を前提にしなくてよい。開発中または個人利用では、`editors/vscode/` を VSIX に package して手元の VS Code に手動 install してよい。

ルール:

- VSIX には runtime に必要な file だけを含めてよい
- test や repo 運用用 metadata を同梱必須にしない
- install 後も実処理は `shosei` CLI に委譲し、拡張単体で publishing logic を持たない

## 7.2 GitHub Release 配布

v0.1 では Marketplace 公開の代わりに、GitHub Release に VSIX asset を添付する配布を許容してよい。

ルール:

- release asset は `shosei-vscode-<version>.vsix` とする
- release は repo 全体の tag `v<shosei-cli-version>` にぶら下げてよい
- release workflow は package 前に extension の構文チェック / test と CLI の check を通す
- 同じ GitHub Release に CLI binary archive が同居してよい
- GitHub Release は VSIX の配布チャネルであり、publish logic の source of truth ではない

## 8. 非目標

- `book.yml` / `series.yml` schema を JS 側で再実装すること
- build / validate / handoff の中身を VS Code extension host に移すこと
- init scaffold を VS Code 側で直接生成すること
- prose / manga の独自 editor を v0.1 で提供すること
- CLI と別系統の状態管理を持つこと
