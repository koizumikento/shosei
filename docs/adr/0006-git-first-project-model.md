# ADR-0006: Git 前提のプロジェクトモデルにする

- Status: Accepted
- Date: 2026-04-12

## Context

バージョン管理は Git 前提にしたいという要望が明示された。書籍制作では本文テキストだけでなく、画像、表紙、ページデータなど履歴管理したい資産が多い。

特に漫画やライトノベルではバイナリ資産の比率が高い。

## Decision

本ツールは Git 前提でプロジェクトを扱う。

最低限の責務:

- `shosei init` で Git 初期化補助
- `.gitignore` 作成
- `.gitattributes` 作成
- build/handoff に commit 情報を残す

推奨事項:

- Git LFS の利用
- バイナリ編集資産の lockable 設定
- handoff 前の dirty worktree 警告

## Consequences

- Git 非利用環境は優先度が下がる
- docs と設定ファイルの差分管理がしやすい
- バイナリ資産を普通の Git だけで運用しない前提が必要になる
