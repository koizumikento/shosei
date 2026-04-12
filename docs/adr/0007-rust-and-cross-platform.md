# ADR-0007: Rust 実装と macOS/Windows/Linux 対応を前提にする

- Status: Accepted
- Date: 2026-04-12

## Context

本ツールは CLI として配布し、macOS, Windows, Linux で使えるようにしたいという要件が追加された。

一方で、本ツールは外部依存として Pandoc, epubcheck, PDF engine, Git などを扱うため、単なるスクリプト集では OS 差異の吸収が難しい。

必要だった条件:

- 単一コマンド体系を 3 OS で維持すること
- シェル固有構文に依存しないこと
- パス差異を吸収すること
- 配布しやすいこと

## Decision

本体は Rust で実装し、macOS / Windows / Linux 向けに OS ごとのネイティブバイナリを提供する。

設計上の前提:

- CLI の主要ロジックは Rust 本体に置く
- 外部ツール呼び出しは Rust からプロセス実行する
- 設定ファイル上のパスは repo-relative かつ `/` 区切りで統一する
- doctor, build, validate の振る舞いは 3 OS で可能な限り揃える

## Consequences

- 3 OS を対象にした CI とスモークテストが必要になる
- 実行ファイル名や PATH 解決など OS 差異を吸収する実装が必要になる
- Windows でも破綻しないパス・文字コード・ログ処理が必要になる
- Bash 前提の補助スクリプトは必須経路に置けなくなる
