# 物語補助仕様 v0.1

作成日: 2026-04-13  
状態: Draft

## 1. 目的

この文書は、`shosei` における物語補助の最小仕様を定義する。

最初の対象は次の通り。

- repo-native な story workspace の生成
- `single-book` と `series` に沿った保存場所の固定
- Markdown / YAML / Git diff に乗る manual-first な運用

## 2. 非目標

- WYSIWYG エディタ
- 本文の自動生成
- NLP だけで暗黙抽出した continuity 判定
- `book.yml` / `series.yml` への story 設定の大量追加

## 3. 設計方針

- 最初は explicit command で opt-in にする
- source of truth は repo 内ファイルとする
- `single-book` と `series` の差は保存場所だけに留める
- 共有 canon と巻固有メモを混ぜない
- file path は repo-relative かつ `/` 区切り前提で運用する

## 4. ディレクトリ規約

### 4.1 `single-book`

```text
repo/
  story/
    README.md
    scenes.yml
    characters/
      README.md
    locations/
      README.md
    terms/
      README.md
    factions/
      README.md
```

### 4.2 `series`

共有 canon:

```text
repo/
  shared/
    metadata/
      story/
        README.md
        characters/
          README.md
        locations/
          README.md
        terms/
          README.md
        factions/
          README.md
```

巻固有:

```text
repo/
  books/
    vol-01/
      story/
        README.md
        scenes.yml
        characters/
          README.md
        locations/
          README.md
        terms/
          README.md
        factions/
          README.md
```

## 5. `shosei story scaffold`

story workspace を生成する。

### 5.1 コマンド形

```bash
shosei story scaffold
shosei story scaffold --book vol-01
shosei story scaffold --shared
shosei story scaffold --force
```

### 5.2 振る舞い

- `single-book`
  - `shosei story scaffold` は `story/` を生成する
  - `--shared` は error
- `series`
  - `shosei story scaffold --shared` は `shared/metadata/story/` を生成する
  - `shosei story scaffold` は対象 book の `books/<book-id>/story/` を生成する
  - repo root から巻固有 scaffold を作る場合は `--book <book-id>` を要求する
  - `books/<book-id>/...` の内側で実行した場合は対象 book を推定できる
- 既存 file は既定で保持する
- `--force` を付けた場合だけ template file を上書きする

### 5.3 生成物

共通:

- `README.md`
- `characters/README.md`
- `locations/README.md`
- `terms/README.md`
- `factions/README.md`

book scope のみ:

- `scenes.yml`

## 6. `scenes.yml` の最小 shape

```yaml
scenes:
  - file: manuscript/01-chapter-1.md
    title: Opening
```

ルール:

- root は mapping とする
- `scenes` は sequence とする
- scene の順序は配列順を正とする
- `file` は repo-relative かつ `/` 区切りの path とする
- `title` は任意
- 不明な key は将来拡張のために許容する

## 7. `shosei story map`

book-scoped な `scenes.yml` を読み、scene 一覧を text と JSON report へ出力する。

```bash
shosei story map
shosei story map --book vol-01
```

v0.1 の最小要件:

- 対象は book-scoped story workspace のみ
- `single-book` では `story/scenes.yml` を読む
- `series` では `books/<book-id>/story/scenes.yml` を読む
- `shared/metadata/story/` は対象外
- report は `dist/reports/<book-id>-story-map.json` に出力する

## 8. `shosei story check`

book-scoped な `scenes.yml` と story entity Markdown を読み、軽い整合チェックを report へ出力する。

```bash
shosei story check
shosei story check --book vol-01
```

v0.1 の最小要件:

- 対象は book-scoped story workspace のみ
- report は `dist/reports/<book-id>-story-check.json` に出力する
- duplicate `file` entry は warning
- invalid `file` path は error
- repo 内に実ファイルが存在しない `file` は warning
- `characters/`, `locations/`, `terms/`, `factions/` 配下の直下 `*.md` を scan 対象にする
- entity ID は frontmatter の `id` を優先し、未指定時は filename stem を使う
- 同一 kind 内で duplicate entity `id` は error
- scene Markdown 冒頭の YAML frontmatter で `characters`, `locations`, `terms`, `factions` を参照配列として読める
- `series` では scene 参照解決時に `books/<book-id>/story/` と `shared/metadata/story/` の両方を対象にする
- 参照先 entity が存在しない場合は warning
- invalid scene/entity frontmatter は error
- shared canon drift や semantic continuity までは扱わない

## 9. `shosei story drift`

`series` の shared canon と巻固有 story data の衝突を report へ出力する。

```bash
shosei story drift --book vol-01
```

v0.1 の最小要件:

- 対象は `series` の book-scoped story workspace のみ
- `shared/metadata/story/` と `books/<book-id>/story/` の両方を読む
- report は `dist/reports/<book-id>-story-drift.json` に出力する
- 同一 tree 内の duplicate entity `id` は error
- `shared` と book-scoped で同じ kind の同じ `id` があり、内容が分岐していれば error
- `shared` と book-scoped で同じ kind の同じ `id` があり、内容が同じなら warning
- scene Markdown や `scenes.yml` は入力に含めない

## 10. 初期内容

- `README.md` は置き場所の意味と運用ルールを短く説明する
- entity directory の `README.md` は 1 file 1 entity の方針と推奨 frontmatter を示す
- scene Markdown では必要なら次の frontmatter を置ける

```yaml
---
characters:
  - hero
locations:
  - school-roof
terms:
  - crimson-key
factions:
  - student-council
---
```

- `characters/`, `locations/`, `terms/`, `factions/` の entity Markdown では `id` を置ける
- `scenes.yml` は空配列の skeleton を置く

## 11. 今後の拡張候補

- repo-scoped story skill template
