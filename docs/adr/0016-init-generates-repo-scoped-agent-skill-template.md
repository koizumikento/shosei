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
- frontmatter の `description` は capability と trigger を短く肯定形で書き、対象外の作業を列挙しない。内容レビューは作品タイプに合う観点を選ぶ
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
  - rewrite ではなく指摘を返すこと
- templates には init 時点の `project.type` と `repo_mode` を埋め込む
- 利用者が後から project 固有ルールを追記しやすいよう、repo note を含める

### 2026-09-13: 継続利用する guidance の見直し

初期巻や初期出力先を command に固定すると、後続巻や別の納品先の依頼を誤誘導しうる。生成する `AGENTS.md` と両 skill では対象巻を依頼または作業ディレクトリから選び、command の `<book-id>` を置き換える。handoff 宛先も依頼から選ぶ。`init` 完了時の次コマンド例は初期巻を指すままとする。

創作を含む運用 skill で既存にない原稿を一律禁止すると、執筆依頼を妨げる。依頼された創作は許容し、事実・出典の捏造と未承認の設定確定を区別して扱う。説明文の除外列挙と本文の重複した対象外リストは取り除く。共通の基本導線と instruction-only の構成は維持する。

参考: [OpenAI Build skills](https://learn.chatgpt.com/docs/build-skills)、[Rethinking skills and prompts](https://learn.chatgpt.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)。

## Consequences

- `shosei` で作った repo を人間と agent が誤った前提で触る確率を下げられる
- `init` 直後から repo-scoped な運用知識を共有できる
- root guidance と skill authoring の初期値を持てるため、利用者は project 固有ルールだけ追記すればよい
- 将来 scripts や app dependency が必要になった場合も、instruction-only skill から段階的に拡張できる
