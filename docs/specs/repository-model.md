# リポジトリ管理モデル v0.1

作成日: 2026-04-12  
状態: Draft

## 1. 目的

この文書は、作品をどの単位で Git リポジトリに載せるかを定義する。

本ツールでは、次の 2 つを正式な管理モデルとする。

- `single-book`
- `series`

## 2. 結論

- 単発作品は `single-book`
- シリーズ作品は `series`
- 無関係な複数作品を 1 repo に混在させない

## 3. モデル一覧

### 3.1 `single-book`

1 冊、または 1 巻を 1 リポジトリとして管理する。

向いているケース:

- 単発の技術書
- 単発のビジネス書
- 単巻完結の小説
- シリーズ共有資産が少ない作品

特徴:

- 構成が単純
- `book.yml` を repo root に置く
- build, validate, handoff がそのまま root で完結する

### 3.2 `series`

シリーズ全体を 1 リポジトリとして管理し、各巻・各冊を子ディレクトリで持つ。

向いているケース:

- ライトノベルシリーズ
- 漫画シリーズ
- 共通資産、共通 CSS、共通世界観資料が多い作品群
- 巻ごとの差分を継続的に管理したい作品

特徴:

- `series.yml` を repo root に置く
- 各巻は `books/<book-id>/book.yml` を持つ
- 共通資産は `shared/` に置く

## 4. 推奨ルール

### `business`

- 既定は `single-book`
- シリーズ教本や継続刊行物で共通資産が強い場合のみ `series`

### `novel`

- 単発なら `single-book`
- 続刊前提なら `series`

### `light-novel`

- 単巻なら `single-book`
- 継続シリーズなら `series` を推奨

### `manga`

- 原則 `series` を推奨
- 読切や短編単体なら `single-book` でもよい

## 5. `single-book` 構成

```text
repo/
  book.yml
  .agents/
    skills/
      shosei-project/
        SKILL.md
  story/
  manuscript/
  manga/
  assets/
  styles/
  dist/
  .gitignore
  .gitattributes
```

備考:

- `project.type` に応じて `manuscript/` か `manga/` を使う
- 物語補助を使う場合は root に `story/` を置く
- 使わないディレクトリは空でもよい

## 6. `series` 構成

```text
repo/
  series.yml
  .agents/
    skills/
      shosei-project/
        SKILL.md
  shared/
    assets/
    styles/
    fonts/
    metadata/
      story/
  books/
    vol-01/
      book.yml
      story/
      manuscript/
      manga/
      assets/
    vol-02/
      book.yml
      story/
      manuscript/
      manga/
      assets/
  dist/
  .gitignore
  .gitattributes
```

備考:

- `shared/` には共通資産だけを置く
- 共通の worldbuilding / canon は `shared/metadata/story/` に置く
- 巻固有資産は `books/<book-id>/assets/` に置く
- 巻固有の scene / note / codex は `books/<book-id>/story/` に置く
- `dist/` は repo root で共通でも、巻ごとに分けてもよい

## 7. root 設定ファイルの役割

### `book.yml`

`single-book` の root 設定。

含むもの:

- 作品固有 metadata
- 出力 profile
- 原稿構成
- validation

### `series.yml`

`series` の root 設定。

含むもの:

- シリーズ名
- シリーズ共通 metadata
- 共有 assets/styles/fonts
- 共通 validation policy
- 巻一覧

補足:

- `books` の並び順はシリーズ内の正順序とみなす
- 将来の `shosei series sync` はこの情報を基準に既刊案内、巻一覧、派生 metadata を生成する

## 8. `series` の継承ルール

`series.yml` と各巻の `book.yml` の関係は次の通り。

- `series.yml`: 共通既定値
- `books/<book-id>/book.yml`: 巻固有の上書き

優先順位:

1. 巻固有 `book.yml`
2. `series.yml`
3. profile の既定値

継承対象:

- 共通 styles
- fonts
- validation policy
- 既定 profile
- Git/LFS 方針

巻固有にすべきもの:

- タイトル
- 巻番号
- 原稿一覧
- ページ数や挿絵
- 巻固有 assets

## 9. CLI 振る舞い

### `single-book`

```bash
shosei build
shosei validate
shosei handoff
```

### `series`

```bash
shosei build --book vol-01
shosei validate --book vol-02
shosei handoff --book vol-03
```

将来:

```bash
shosei build --all
shosei validate --all
shosei series sync
```

`shosei series sync` の方針:

- `series.yml` を正として巻一覧、巻番号、既刊案内を同期する
- 手書き原稿本文を直接 rewrite するのではなく、派生 metadata や backmatter 生成を優先する
- 巻固有 `book.yml` の明示 override は保持する

## 10. `init` の扱い

`shosei init` では `repo_mode` を質問する。

- `single-book`: root に `book.yml` を作る
- `series`: root に `series.yml` を作り、最初の巻として `books/<book-id>/book.yml` も作る

後から `single-book` を `series` に昇格させる手順は [single-book から series への移行仕様](repository-migration.md) を参照する。

## 11. Git 運用上の利点

### `single-book`

- commit が冊単位で明快
- リリースタグが切りやすい
- 完結作品のアーカイブに向く

### `series`

- 共通資産の重複を減らせる
- 巻間差分を比較しやすい
- CI や handoff をシリーズ単位で運用しやすい
- 漫画やラノベの継続運用に向く

## 12. 避けるべき構成

- 無関係な複数シリーズを 1 repo に混在させる
- シリーズなのに巻ごとに別 repo にして共通資産をコピーする
- `shared/` に巻固有データを入れる

## 13. v0.1 の決定

- 正式サポートは `single-book` と `series`
- `multi-series` は未対応
- 既定値は `single-book`
- `manga` では `series` を推奨
- 後からのシリーズ化を前提にする
