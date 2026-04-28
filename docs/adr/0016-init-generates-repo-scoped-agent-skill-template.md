# ADR-0016: `shosei init` で repo guidance と repo-scoped agent skill templates を生成する

- Status: Accepted
- Date: 2026-04-13

## Context

`shosei` で初期化したリポジトリでは、設定確認、原稿編集、内容レビュー、`validate` / `build` / `preview` / `handoff` の繰り返し手順が発生する。

一方で、生成 AI や coding agent に同じ運用ルールを毎回 prompt で説明すると、`single-book` / `series` の違い、`explain` を先に使う方針、`book.yml` / `series.yml` の安定名などの durable rule が会話ごとに抜けやすい。

root の `AGENTS.md` は repo-wide な運用ルールを共有する場所として使える。一方で、Codex の公式ドキュメントでは、繰り返し使う手順は repo-scoped な `.agents/skills/` 配下の skill に切り出し、1 skill 1 job、instruction-first、明確な description を推奨している。

## Decision

`shosei init` は、初期 scaffold の一部として root `AGENTS.md` と repo-scoped agent skill templates を生成する。

ルール:

- root `AGENTS.md` は `shosei` CLI を使うための repo-wide guidance として生成する
- `AGENTS.md` には init 時点の `project.type` と `repo_mode`、`shosei explain` を先に使う方針、`validate` / `build` / `preview` / `handoff` の基本導線、`series` での `--book <book-id>` 利用ルール、config path を repo-relative かつ `/` 区切りで保つルールを含める
- 出力先は repo root の `.agents/skills/shosei-project/SKILL.md` と `.agents/skills/shosei-content-review/SKILL.md`
- skill は instruction-only を既定とし、`scripts/`, `references/`, `agents/openai.yaml` は生成しない
- `shosei-project` の責務は「`shosei` 管理下の出版リポジトリを運用すること」に絞る
- `shosei-content-review` の責務は「`shosei` 管理下の manuscript / editorial / story / reference / proof packet を内容レビューすること」に絞る
- frontmatter の `description` には trigger になりやすい語を含める
- `shosei-project` の本文には少なくとも次を含める
  - `single-book` / `series` の見分け方
  - `series` での `--book <book-id>` 利用ルール
  - `shosei explain` を先に使う方針
  - `validate` / `build` / `preview` / `handoff` の基本導線
  - 設定 path は repo-relative かつ `/` 区切りで保つこと
- `shosei-content-review` の本文には少なくとも次を含める
  - manuscript, editorial, story, reference, proof packet を対象にすること
  - reference workspace がある場合は `reference map` を先に使い、source-backed review では reference entry を主要な review aid として扱うこと
  - `series` で reference を使う review では book-scoped と shared の scope を見分け、必要なら `reference drift` で source of truth の衝突を確認すること
  - findings-first で内容上の問題や review readiness を見ること
  - コードレビューではないこと
  - rewrite ではなく指摘を返すこと
- templates には init 時点の `project.type` と `repo_mode` を埋め込む
- 利用者が後から project 固有ルールを追記しやすいよう、repo note を含める

## Consequences

- `shosei` で作った repo を人間と agent が誤った前提で触る確率を下げられる
- `init` 直後から repo-scoped な運用知識を共有できる
- root guidance と skill authoring の初期値を持てるため、利用者は project 固有ルールだけ追記すればよい
- 将来 scripts や app dependency が必要になった場合も、instruction-only skill から段階的に拡張できる
