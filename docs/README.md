# Docs

- [Usage ガイド](usage.md)
- [設定リファレンス](config-reference.md)
- [機能仕様書](specs/functional-spec.md)
- [リポジトリ管理モデル](specs/repository-model.md)
- [single-book から series への移行仕様](specs/repository-migration.md)
- [設定ファイル schema](specs/config-schema.md)
- [series.yml schema](specs/series-schema.md)
- [設定探索と継承ルール](specs/config-loading.md)
- [Rust 実装アーキテクチャ](specs/rust-architecture.md)
- [VS Code 拡張仕様](specs/vscode-extension.md)
- [init ウィザード仕様](specs/init-wizard.md)
- [物語補助仕様](specs/story-support.md)
- [参考資料ワークスペース仕様](specs/reference-workspace.md)
- [参考資料ワークスペース採用判断ポイント](specs/reference-workspace-decision-points.md)
- [ADR 一覧](adr/README.md)

このディレクトリでは、現行の product contract を表す仕様と、意思決定の履歴を分けて管理する。

- `specs/`: 現行 contract と、その周辺の仕様整理
- `adr/`: なぜその方針を採ったか、後から見返すための決定記録

現行の product contract に追随している spec は、版表記を `v0.2`、状態を `Current` として扱う。検討過程のメモや置き換え済み文書は、その状態を明示したまま残す。
