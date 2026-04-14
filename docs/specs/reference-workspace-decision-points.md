# 参考資料ワークスペース採用判断ポイント

作成日: 2026-04-14  
状態: Superseded by ADR-0027

この文書は、参考資料ワークスペースを ADR 化する前に、何を先に決めるべきかを短く整理するためのメモである。

採用判断は [ADR-0027](../adr/0027-reference-workspace-starts-as-an-explicit-opt-in-surface.md) に反映済みであり、現在の正式方針は ADR と [参考資料ワークスペース仕様](reference-workspace.md) を参照する。

## 1. 先に決める論点

### 1.1 対象範囲

候補:

- 全 project type を対象にする
- prose 系だけを対象にする
- `story` を使う project だけを対象にする

判断軸:

- 参考リンクとメモは build pipeline ではなく制作過程の補助情報か
- 漫画や論文でも同じ置き場が自然か
- `editorial` や `story` に寄せた方が意味が明確か

現時点の推奨:

- 全 project type を対象にする
- prose 固有の review metadata は `editorial` に残す
- 物語固有の codex / canon は `story` に残す

### 1.2 surface の持ち方

候補:

- 新しい `reference` surface を作る
- `story` の note / codex 拡張として扱う
- `editorial` sidecar の一部として扱う

判断軸:

- 名前だけで責務が伝わるか
- prose 以外でも無理なく使えるか
- 既存 surface の責務を濁さないか

現時点の推奨:

- 新しい `reference` surface を作る

### 1.3 保存場所

候補:

- `single-book`: `references/`
- `series` shared: `shared/metadata/references/`
- `series` book-scoped: `books/<book-id>/references/`

判断軸:

- `story` と同じ shared / book-scoped 分離を維持できるか
- `shared/` は共通資産だけを置くという repo model に沿うか
- root 設定を増やさずに済むか

現時点の推奨:

- 上記の 3 か所で固定する

### 1.4 v0.1 の entry shape

候補:

- Markdown 本文 + YAML frontmatter
- YAML only
- Markdown 本文 + inline link list

判断軸:

- Git diff で読みやすいか
- メモ本文と structured field を両立できるか
- 将来 `map` / `check` を追加しやすいか

現時点の推奨:

- Markdown 本文 + YAML frontmatter
- 1 file 1 entry

### 1.5 最初の CLI 境界

候補:

- `shosei reference scaffold` だけ入れる
- `scaffold` と `map` まで入れる
- `scaffold` / `map` / `check` を最初から揃える

判断軸:

- workspace の保存場所を固定するだけで十分価値が出るか
- v0.1 で validation まで背負うと scope が広がりすぎないか
- docs と VS Code adapter の同期コストが増えすぎないか

現時点の推奨:

- まずは `shosei reference scaffold`
- `map` と `check` は次段に分ける

### 1.6 v0.1 で何を検証しないか

未対応候補:

- URL の疎通確認
- Web 内容の取得
- 本文や `editorial.claims.yml` との自動照合
- shared / book-scoped 間の自動同期

現時点の推奨:

- v0.1 は保存場所と file shape の固定に留める

## 2. ADR に進めるための最小合意

次の 5 点に合意できれば、ADR と最小実装仕様に落とせる。

1. 参考資料ワークスペースは全 project type を対象にする
2. `story` / `editorial` とは別の `reference` surface として持つ
3. 保存場所は `references/`, `shared/metadata/references/`, `books/<book-id>/references/` で固定する
4. entry は Markdown 1 file + frontmatter で表す
5. 最初の CLI は `shosei reference scaffold` のみに留める

## 3. いま保留してよい論点

- `status` を enum で厳格化するか
- `related_sections` を prose 以外にも広げるか
- `editorial.claims.yml` の `sources` と reference entry を相互参照させるか
- shared reference と巻固有 reference の drift を検出するか
- VS Code 側に専用ビューを作るか
