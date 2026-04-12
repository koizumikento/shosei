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

記法:

- `Status`: `Proposed`, `Accepted`, `Superseded`
- 後から方針を変えた場合は、新しい ADR を追加し、古いものは `Superseded` にする
