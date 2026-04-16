# Structures

この directory は book-scoped な構成メモ置き場です。

- `kishotenketsu.md`: 起承転結
- `three-act.md`: 三幕構成
- `save-the-cat.md`: Save the Cat! 15ビート
- `heroes-journey.md`: ヒーローズ・ジャーニー

使い方:

- まずは近い型を 1 つ選んでそのまま埋める
- 比較案を作りたいときは複製して別名で置く
- 見出しや項目は作品に合わせて増減してよい
- `scene_seeds` frontmatter を編集してから `shosei story seed --template <name>` を実行すると、`scenes.yml` と `scene-notes/*.md` の叩き台を起こせる
- `scene_seeds[*].file` を省略した場合は `scene-notes/` 配下へ自動採番する
- v0.1 の CLI は本文全体は解釈せず、`story seed` 用に `scene_seeds` frontmatter だけを読む
