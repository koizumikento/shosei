# ADR-0025: VS Code 拡張は `shosei` CLI を呼び出す薄いアダプタにする

- Status: Accepted
- Date: 2026-04-14

## Context

`shosei` の日常利用では、`explain` / `validate` / `build` / `preview` / `page check` / `series sync` を繰り返す。VS Code からこれらを実行しやすくしたいが、repo discovery、config merge、prose / manga の分岐、外部ツール連携を editor 側に複製すると、Rust 実装と挙動がズレやすい。

一方で VS Code extension host は Node.js 上で動くため、拡張そのものは Rust 単体では完結しない。

## Decision

VS Code 拡張は `editors/vscode/` 配下に置き、`shosei` CLI を外部プロセスとして呼び出す薄いアダプタとして実装する。

ルール:

- 実処理の source of truth は `shosei` CLI と `shosei-core`
- VS Code 側で repo discovery や config schema を再実装しない
- `validate` と `page check` の Problems 反映には、CLI が生成する JSON report を使う
- `series` で必要な `book-id` は、専用ビューで選んだ値、`books/<book-id>/...` 配下の editor context、設定値、Quick Pick の順で解決する
- `preview --watch` は VS Code terminal task で動かす
- extension host の JavaScript は VS Code API と CLI orchestration に限定する

## Consequences

- 出版ロジックの変更は引き続き Rust 側を更新すればよく、editor 側の追従コストを下げられる
- CLI がすでに生成している report を活用できるため、拡張側で独自の検証器を持たずに Problems を出せる
- VS Code API の都合で JavaScript を導入するが、コア仕様と処理系は Rust に残る
- 将来ほかの editor integration を追加しても、同じ shell-out パターンを再利用しやすい
