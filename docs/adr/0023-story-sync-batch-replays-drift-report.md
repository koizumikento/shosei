# ADR-0022: story sync の batch 適用は drift report 駆動で扱う

- Status: Accepted
- Date: 2026-04-14

## Context

`story sync` は `--from shared` / `--to shared` による 1 entity copy までは扱えるようになったが、`story drift` で複数の衝突が見つかったときは同じ操作を繰り返す必要があった。

一方で、`story sync` 自身が shared/book の差分を再探索し始めると、`story drift` との責務境界が曖昧になり、どの時点の差分を適用したのかも追いにくくなる。

## Decision

`story sync` の batch 適用は、`story drift` report を明示入力に取る形で扱う。

ルール:

- `story drift` report は machine-readable な `drifts` 配列を持つ
- `story sync --report <path>` はその `drifts` 配列だけを入力にする
- report mode でも `--from shared` か `--to shared` の方向指定は必須にする
- report mode は `--force` を必須にする
- report mode は report に含まれる `shared_path` / `book_path` をそのまま使い、対象 entity 群を再探索しない
- `scenes.yml` の更新や automatic merge は行わない

## Consequences

- `story drift` で見つかった差分を、どの report を使ってどちら向きに反映したかを明示できる
- bulk repair を入れても、差分検出は `story drift`、反映は `story sync` という責務分離を保てる
- report schema は少し増えるが、後続の自動化や review がしやすくなる
