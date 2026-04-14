# ADR-0017: `shosei chapter` は prose の source structure だけを更新する

- Status: Accepted
- Date: 2026-04-13

## Context

`shosei` には次の 2 つの独立した軸がある。

- repo mode
  - `single-book`
  - `series`
- manuscript model
  - prose
  - manga

repo mode は対象 `book.yml` の解決方法を決める。
一方で manuscript model は source structure と build / validate の前提を決める。

既存仕様では、prose は `manuscript.frontmatter`, `manuscript.chapters`, `manuscript.backmatter` を source structure とし、manga は `manga/pages/` の page order を primary source とする。

今後 `shosei chapter add|move|remove` を追加するにあたり、次が曖昧なままだと混同が起きる。

- `series` 対応と prose / manga 対応を同じ論点として扱ってしまう
- `01-`, `02-` などの filename prefix を章順の根拠にしてしまう
- `move` が config 更新なのか file rename / renumber なのかがぶれる

## Decision

`shosei chapter add|move|remove` は prose book の source structure mutator として定義する。

ルール:

- 対象は `project.type != manga` の book に限る
- `single-book` と `series` の違いは対象 book の解決方法だけに留める
- `chapter` は対象 book の `book.yml` にある `manuscript.chapters` を更新する
- prose の章順は `manuscript.chapters` の配列順を正とする
- filename prefix は見た目や既存 scaffold との互換のための命名慣例であり、順序の根拠にはしない
- `move` は既定で file rename や renumber を行わない
- `remove` は既定で config から外すだけに留め、物理削除は明示 opt-in にする
- filename prefix を整えたい場合は、順序変更とは別責務の明示 `renumber` コマンドとして扱う
- `manga` の page order、chapter / episode metadata、`manga/pages/` の操作は `chapter` コマンドの責務に含めない

## Consequences

- `series` 対応は repo discovery の問題として閉じ、prose / manga の構造差と混同しにくくなる
- 章順の変更は YAML 配列の再配置だけで成立し、filename prefix の renumber を強制しない
- prefix 整形は opt-in の別操作として追加でき、順序変更と file rename を分離できる
- scaffold が生成する `01-chapter-1.md` 形式は維持できるが、順序の source of truth は config に一本化される
- `manga` 側は page-based model をそのまま維持できる
- 将来、見た目の prefix を整える `renumber` 系コマンドを別責務として追加しやすくなる
