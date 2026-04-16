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
    scene-notes/
      01-scene.md
    structures/
      README.md
      kishotenketsu.md
      three-act.md
      save-the-cat.md
      heroes-journey.md
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
        scene-notes/
          01-scene.md
        structures/
          README.md
          kishotenketsu.md
          three-act.md
          save-the-cat.md
          heroes-journey.md
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
- `characters/_template.md`
- `locations/README.md`
- `locations/_template.md`
- `terms/README.md`
- `terms/_template.md`
- `factions/README.md`
- `factions/_template.md`

book scope のみ:

- `scenes.yml`
- `scene-template.md`
- `structures/README.md`
- `structures/kishotenketsu.md`
- `structures/three-act.md`
- `structures/save-the-cat.md`
- `structures/heroes-journey.md`

補足:

- entity directory の `_template.md` は scaffold に含まれる予約 file とし、`story check` / `story drift` / `story sync` の scan 対象から除外する
- scaffold される template/README は日本語中心で説明してよい
- parser が意味を持って読む key は canonical な英語 key に固定する
- `structures/` は book-scoped な構成メモ置き場とし、v0.1 の CLI は本文全体は解釈しないが `story seed` 用に `scene_seeds` frontmatter は読む

## 6. `shosei story seed`

book-scoped な structure template の `scene_seeds` を使って `scenes.yml` と scene note を生成する。

### 6.1 コマンド形

```bash
shosei story seed --template kishotenketsu
shosei story seed --template three-act --book vol-01
shosei story seed --template save-the-cat --force
```

### 6.2 振る舞い

- 対象は book scope のみ
- `single-book` では `story/structures/<template>.md` を読む
- `series` では `books/<book-id>/story/structures/<template>.md` を読む
- `--template` は `structures/` 配下の file stem または `.md` 付き file 名を受け付ける
- structure template の frontmatter root は YAML mapping とし、`scene_seeds` sequence を持つ
- 各 `scene_seeds` entry は少なくとも `title` を持つ
- `scene_seeds[*].file` を省略した場合は `story/scene-notes/<nn>-scene.md` 相当の repo-relative path を自動採番する
- `scene_seeds[*].file` を指定した場合は repo-relative かつ `/` 区切りの Markdown path とする
- command は `scenes.yml` を seed 順で書き換える
- command は scene note file が無ければ作成する
- 既存の scene note file は既定で保持し、`--force` を付けた場合だけ seed 内容で上書きする
- `scenes.yml` が非空で seed 内容と異なる場合、`--force` を要求する

### 6.3 `scene_seeds` の最小 shape

```yaml
scene_seeds:
  - title: 起: 日常の提示
    beat: 起
    summary: 主人公の日常と物語の約束を見せる
    characters:
      - 主人公
```

ルール:

- `title` は必須の non-empty string
- `beat`, `summary`, `file` は任意の string
- `characters`, `locations`, `terms`, `factions` は string または string sequence
- 不明な key は将来拡張のために許容する
- 生成される scene note frontmatter では canonical key `characters`, `locations`, `terms`, `factions` を使う
- 生成される scene note frontmatter に `structure_template`, `structure_beat` のような補助 key を追加してよい

## 7. `scenes.yml` の最小 shape

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
- scene entry key は `file` / `title` を使う
- `title` は任意
- 不明な key は将来拡張のために許容する

## 8. `shosei story map`

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
- report は `single-book` では `dist/reports/default-story-map.json`、`series` では `dist/reports/<book-id>-story-map.json` に出力する

## 9. `shosei story check`

book-scoped な `scenes.yml` と story entity Markdown を読み、軽い整合チェックを report へ出力する。

```bash
shosei story check
shosei story check --book vol-01
```

v0.1 の最小要件:

- 対象は book-scoped story workspace のみ
- report は `single-book` では `dist/reports/default-story-check.json`、`series` では `dist/reports/<book-id>-story-check.json` に出力する
- duplicate `file` entry は warning
- invalid `file` path は error
- repo 内に実ファイルが存在しない `file` は warning
- `characters/`, `locations/`, `terms/`, `factions/` 配下の直下 `*.md` を scan 対象にする
- ただし `README.md` と `_template.md` は scan 対象から除外する
- entity ID は frontmatter の `id` を優先し、未指定時は filename stem を使う
- 同一 kind 内で duplicate entity `id` は error
- scene Markdown 冒頭の YAML frontmatter で `characters`, `locations`, `terms`, `factions` を参照配列として読む
- `series` では scene 参照解決時に `books/<book-id>/story/` と `shared/metadata/story/` の両方を対象にする
- 参照先 entity が存在しない場合は warning
- invalid scene/entity frontmatter は error
- shared canon drift や semantic continuity までは扱わない

## 10. `shosei story drift`

`series` の shared canon と巻固有 story data の衝突を report へ出力する。

```bash
shosei story drift --book vol-01
```

v0.1 の最小要件:

- 対象は `series` の book-scoped story workspace のみ
- `shared/metadata/story/` と `books/<book-id>/story/` の両方を読む
- report は `dist/reports/<book-id>-story-drift.json` に出力する
- report には machine-readable な `drifts` 配列を含める
- 各 `drifts` entry は `kind`, `id`, `status`, `shared_path`, `book_path` を持つ
- 同一 tree 内の duplicate entity `id` は error
- `shared` と book-scoped で同じ kind の同じ `id` があり、内容が分岐していれば error
- `shared` と book-scoped で同じ kind の同じ `id` があり、内容が同じなら warning
- scene Markdown や `scenes.yml` は入力に含めない

## 11. `shosei story sync`

`series` で shared canon と巻固有 story workspace の間を明示コピーする。単体 copy と、`story drift` report に基づく batch copy を両方サポートする。

```bash
shosei story sync --book vol-01 --from shared --kind character --id lead
shosei story sync --book vol-01 --to shared --kind character --id lead
shosei story sync --book vol-01 --from shared --kind character --id lead --force
shosei story sync --book vol-01 --to shared --kind character --id lead --force
shosei story sync --book vol-01 --from shared --report dist/reports/vol-01-story-drift.json --force
shosei story sync --book vol-01 --to shared --report dist/reports/vol-01-story-drift.json --force
```

v0.1 の最小要件:

- 対象は `series` のみ
- `--from shared` か `--to shared` のどちらか一方だけを受け付ける
- 単体 mode では対象 entity を `--kind` と `--id` で 1 件ずつ指定する
- report mode では `--report <story-drift-report>` を受け付け、`--kind` と `--id` は受け付けない
- report mode は `--force` を必須にする
- `--from shared` のとき:
  - source は `shared/metadata/story/<kind>/`
  - destination は `books/<book-id>/story/<kind>/`
  - book 側に同じ `id` が無ければ source file 名を保って copy する
  - book 側に同じ `id` があり内容が違う場合、既定では error、`--force` のときだけ上書きする
  - book 側に同じ `id` があり内容も同じ場合は no-op summary を返す
- `--to shared` のとき:
  - source は `books/<book-id>/story/<kind>/`
  - destination は `shared/metadata/story/<kind>/`
  - shared 側に同じ `id` が無ければ source file 名を保って copy する
  - shared 側に同じ `id` があり内容が違う場合、既定では error、`--force` のときだけ上書きする
  - shared 側に同じ `id` があり内容も同じ場合は no-op summary を返す
- report mode は `story drift` report の `drifts` 配列に含まれる全 entry を、指定方向へ順に適用する
- report mode は report 内の `shared_path` / `book_path` をそのまま使い、対象セットの再発見はしない
- `scenes.yml` は更新しない

## 12. 初期内容

- `README.md` は置き場所の意味と運用ルールを短く説明する
- entity directory の `README.md` は 1 file 1 entity の方針、`_template.md` の使い方、CLI が読む canonical key を示す
- entity directory の `_template.md` は日本語中心の記入例を置く
- book scope の `scene-template.md` は日本語中心の scene frontmatter 記入例を置く
- book scope の `structures/README.md` は各構成テンプレートの使い分けを案内する
- book scope の `structures/*.md` は `起承転結`、`三幕構成`、`Save the Cat! 15ビート`、`ヒーローズ・ジャーニー` の叩き台を置く
- book scope の `structures/*.md` は `scene_seeds` frontmatter を持ち、`story seed` の入力にもなる
- `story seed` が作る `scene-notes/*.md` は scene frontmatter の下書きと構成メモを含む
- machine-read key は `id`, `characters`, `locations`, `terms`, `factions`, `scenes`, `file`, `title` のように英語のまま使う
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

## 13. 今後の拡張候補

- repo-scoped story skill template
