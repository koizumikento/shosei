# ADR

Architecture Decision Record の一覧。

- [ADR-0001: 日本語出版向け CLI を主対象にする](0001-japanese-publishing-scope.md)
- [ADR-0002: prose は Pandoc を中核にする](0002-pandoc-centered-prose-pipeline.md)
- [ADR-0003: 設定ファイル中心の優しい CLI にする](0003-config-first-cli.md)
- [ADR-0004: 縦書き・横書きと target profile を原稿モデルに組み込む](0004-writing-mode-and-target-profiles.md)
- [ADR-0005: 漫画を別原稿モデルとして扱う](0005-manga-as-separate-manuscript-model.md)
- [ADR-0006: Git 前提のプロジェクトモデルにする](0006-git-first-project-model.md)
- [ADR-0007: Rust 実装と macOS/Windows/Linux 対応を前提にする](0007-rust-and-cross-platform.md)
- [ADR-0008: リポジトリ管理単位は single-book と series を正式サポートする](0008-repository-unit-single-book-and-series.md)
- [ADR-0009: 設定探索は上方探索、継承優先順位は CLI > book.yml > series.yml > profile defaults とする](0009-config-discovery-and-precedence.md)
- [ADR-0010: single-book から series への移行パスを前提にする](0010-single-book-to-series-migration.md)
- [ADR-0011: CLI 名は shosei を採用する](0011-cli-name-shosei.md)
- [ADR-0012: 価値の重心を preflight・explain・handoff に置く](0012-preflight-explain-and-handoff-priority.md)
- [ADR-0013: manga のページ順は `manga/pages/` の辞書順を既定にする](0013-manga-page-discovery-fallback.md)
- [ADR-0014: 外部カバー画像は `book.yml` で明示し、本文ページと分離する](0014-cover-asset-separation.md)
- [ADR-0015: navigation 構造は source / semantic / navigation で分離する](0015-navigation-structure-separation.md)
- [ADR-0016: `shosei init` で repo-scoped agent skill templates を生成する](0016-init-generates-repo-scoped-agent-skill-template.md)
- [ADR-0017: `shosei chapter` は prose の source structure だけを更新する](0017-chapter-commands-follow-prose-source-structure.md)
- [ADR-0018: prose 向け editorial metadata は sidecar file で扱い、`handoff proof` に review packet を含める](0018-editorial-sidecars-and-proof-packets.md)
- [ADR-0019: 物語補助は explicit な repo-native scaffold から始める](0019-story-support-starts-with-explicit-repo-native-scaffold.md)
- [ADR-0020: story drift は series 向けの別コマンドとして扱う](0020-story-drift-is-a-separate-series-command.md)
- [ADR-0021: story sync は explicit な shared-to-book copy から始める](0021-story-sync-starts-as-explicit-shared-to-book-copy.md)
- [ADR-0022: story sync は explicit のまま 1 entity の双方向 copy を許可する](0022-story-sync-stays-explicit-but-supports-both-directions.md)
- [ADR-0023: story sync の batch 適用は drift report 駆動で扱う](0023-story-sync-batch-replays-drift-report.md)
- [ADR-0024: 論文と発表前刷りは prose 系として扱い、発表前刷りは print layout preset で表す](0024-paper-and-preprint-stay-in-prose.md)
- [ADR-0025: VS Code 拡張は `shosei` CLI を呼び出す薄いアダプタにする](0025-vscode-extension-shells-out-to-cli.md)
- [ADR-0026: prose print の v0.1 PDF backend は weasyprint を正式採用する](0026-prose-print-uses-weasyprint-in-v0.1.md)
- [ADR-0027: 参考資料ワークスペースは独立した opt-in surface として導入する](0027-reference-workspace-starts-as-an-explicit-opt-in-surface.md)
- [ADR-0028: `claims.yml` の reference source は `ref:<id>` で明示する](0028-claims-source-ref-prefix-links-editorial-and-reference.md)
- [ADR-0029: prose の default design は scaffolded stylesheet で持ち、build 時に適用する](0029-prose-default-design-lives-in-scaffolded-stylesheets.md)
- [ADR-0030: 縦組み prose print は Chromium backend を使う](0030-vertical-prose-print-uses-chromium.md)
- [ADR-0031: Kindle Previewer validation is opt-in](0031-kindle-previewer-validation-is-opt-in.md)

記法:

- `Status`: `Proposed`, `Accepted`, `Superseded`
- 後から方針を変えた場合は、新しい ADR を追加し、古いものは `Superseded` にする
