# ADR-0012: 価値の重心を preflight・explain・handoff に置く

- Status: Accepted
- Date: 2026-04-13

## Context

電子書籍・書籍制作の周辺には、既に EPUB/PDF の生成や執筆 UI を提供するツールが多い。

一方で、日本語出版の実務では次が繰り返し問題になる。

- 設定継承が見えず、どの値が効いているか分かりにくい
- Kindle / 印刷 / EPUB Accessibility の提出前確認が分散しやすい
- レイアウト確認の反復が遅い
- series 運用で巻一覧や既刊案内がずれやすい
- manga でページ順、見開き、panel metadata の不整合が起きやすい
- 外部校正や編集者に渡す成果物を毎回手でまとめがち

`shosei` が単なる変換器や執筆 UI を追うだけでは、既存ツールとの差別化が弱い。

## Decision

`shosei` の近接価値は、変換器そのものより制作フローの制御と提出前品質保証に置く。

優先して仕様化・実装検討する機能は次とする。

1. `shosei explain`
   - 最終有効設定と値の由来を表示する
2. `shosei validate`
   - lint ではなく target 別 preflight として強化する
3. `shosei preview --watch`
   - 変更監視しながら preview を継続更新する
4. `shosei series sync`
   - `series.yml` を正として巻一覧、既刊案内、派生 metadata を同期する
5. `shosei page check`
   - manga のページ順、見開き、panel metadata を検査する
6. `shosei handoff proof`
   - 校正・編集向けの成果物パッケージを生成する

## Consequences

- CLI の価値は format conversion 単体より、explainability、preflight、handoff に寄る
- 既存コマンドの責務は深くなるが、日常コマンド数はむやみに増やさない
- `series` 補助機能は `series.yml` を正とする
- `page check` や `series sync` は、手書き原稿を無断で大きく rewrite しない保守的方針を取る
- ストアへの直接アップロード、WYSIWYG、リアルタイム共同編集は後順位のまま残す
