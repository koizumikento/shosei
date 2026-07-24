# ADR-0035: VS Code 拡張は platform 別 `shosei` CLI を同梱する

- Status: Accepted
- Date: 2026-07-24

## Context

ADR-0025 により、VS Code 拡張は出版ロジックを再実装せず、`shosei` CLI を外部プロセスとして呼び出す薄いアダプタになっている。

この境界は CLI と editor integration の挙動を揃えやすい一方、拡張利用者が `shosei` CLI を別途 install しなければならない。release workflow はすでに macOS、Windows、Linux 向け CLI binary を生成しており、VS Code は platform-specific VSIX を配布できる。

Wasm 版 core を拡張へ組み込む案もあるが、現在の `build`、`doctor`、external validator は host filesystem と外部 process 起動を必要とする。別 install の解消だけを目的に Wasm host adapter を導入すると、CLI と同等の process / filesystem orchestration を新たに維持する必要がある。

## Decision

VS Code 拡張は、対応する platform-specific VSIX に同じ release version の `shosei` CLI binary を同梱する。

ADR-0025 の shell-out 境界は維持する。拡張は同梱 CLI も外部 process として実行し、出版ロジック、repo discovery、config merge、validation planning、toolchain inspection を JavaScript 側へ移さない。

CLI runner は次の順で解決する。

1. `shosei.cli.command` / `shosei.cli.args` で明示された external runner
2. Extension Development Host の repo-local `cargo run` fallback
3. platform-specific VSIX に同梱された CLI
4. 同梱 CLI がない universal VSIX または未対応 platform では `PATH` 上の `shosei`

初期の bundled target は、既存 release binary と揃える。

- `linux-x64`
- `darwin-x64`
- `darwin-arm64`
- `win32-x64`

対応 target 以外にも拡張を install できるよう、CLI binary を含まない universal VSIX を fallback package として維持する。

Pandoc、Chromium、epubcheck、qpdf、Kindle Previewer など、`shosei` が呼び出す外部 toolchain は同梱しない。これらの検出と案内は引き続き `shosei doctor` が所有する。

## Consequences

- 対応 platform では VS Code 拡張だけで `shosei` CLI 本体を利用できる
- 拡張と bundled CLI を同じ VSIX version として更新できる
- CLI 単体配布、Homebrew、Scoop、source install は継続できる
- custom build や source checkout は `shosei.cli.command` / `shosei.cli.args` で引き続き指定できる
- release workflow は platform ごとの CLI binary と VSIX を対応付け、version 一致と package smoke を検証する必要がある
- manual VSIX install の update 動作は editor の既存挙動に従い、registry 経由の自動 update とは区別して案内する
- Wasm 化は Web Extension など別の目的が具体化した場合に改めて判断する
