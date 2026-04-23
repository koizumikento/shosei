# 設定探索と継承ルール v0.2

作成日: 2026-04-12  
状態: Current

## 1. 目的

この文書は、`book.yml` と `series.yml` をどのように探索し、どの順で解決し、どの値を優先するかを定義する。

対象:

- repo root の探索
- `single-book` / `series` の判定
- 対象 book の解決
- 設定の継承と merge
- パスの正規化

## 2. 基本方針

- CLI は現在作業ディレクトリから repo root を自動発見する
- `book.yml` と `series.yml` の両方がある場合は、より具体的な `book` コンテキストを優先する
- `series` では root 設定を共通既定値として扱い、各巻の `book.yml` で上書きする
- merge 規則は予測可能性を優先し、特殊扱いを増やさない

## 3. 用語

- `repo root`: `.git`, `book.yml`, `series.yml` などを起点に見つかるリポジトリの基準ディレクトリ
- `book root`: 1 冊分の `book.yml` が存在するディレクトリ
- `series root`: `series.yml` が存在するディレクトリ
- `book context`: 実行対象となる 1 冊分の解決済みコンテキスト
- `RepoPath`: config 上の `/` 区切り相対パス
- `FsPath`: 実行 OS 上の実際のファイルシステムパス

## 4. repo root 探索

### 4.1 入力

探索開始地点は次のいずれか。

1. `--path` が指定された場合はそのパス
2. それ以外は現在作業ディレクトリ

### 4.2 探索ルール

開始地点から親ディレクトリへ向かって上にたどり、次を確認する。

- `book.yml`
- `series.yml`
- `.git`

### 4.3 判定ルール

1. 同一ディレクトリに `book.yml` のみある場合:
   - `single-book` と判定
2. 同一ディレクトリに `series.yml` のみある場合:
   - `series` と判定
3. `books/<book-id>/book.yml` があり、その上位に `series.yml` がある場合:
   - `series` と判定し、その `book.yml` を対象 book とみなす
4. 同一ディレクトリに `book.yml` と `series.yml` が両方ある場合:
   - v0.2 では error
5. 上方探索でどちらも見つからない場合:
   - `not initialized` として error

## 5. book context の解決

### 5.1 `single-book`

対象 book は repo root の `book.yml` で一意に決まる。

### 5.2 `series`

対象 book は次の順で決める。

1. `--book <book-id>` が指定された場合
2. 現在位置が `books/<book-id>/` 配下にある場合
3. それ以外は未解決

未解決時の扱い:

- `shosei build`, `shosei validate`, `shosei handoff`, `shosei preview` は error
- 将来 `--all` 対応コマンドは全巻対象で実行可

## 6. 設定ソースの優先順位

解決済み設定の優先順位は以下とする。

1. CLI override
2. 巻固有 `book.yml`
3. `series.yml`
4. profile の既定値
5. コード上の最終 fallback

補足:

- v0.2 では CLI override は限定的に扱う
- `series.yml` は `defaults` 以下を通して巻へ既定値を供給する

## 7. merge ルール

### 7.1 scalar

後勝ちで上書きする。

例:

- `book.title`
- `book.writing_mode`
- `print.trim_size`

### 7.2 object

キー単位で再帰的に merge する。

例:

- `outputs`
- `validation`
- `git`

### 7.3 array

v0.2 では基本的に **置換** とする。

例:

- `manuscript.chapters`
- `manuscript.frontmatter`
- `sections`
- `git.lockable`

理由:

- append merge は出所が追いにくく、意図しない重複を生みやすい
- `manuscript.chapters` は配列順そのものが prose の source structure になる
- loader や CLI は filename prefix で chapter 順を再解釈しない
- `shosei chapter add|move|remove` はこの配列を保守的に更新する

### 7.4 例外: 共有探索パス

`series` の `shared.assets`, `shared.styles`, `shared.fonts`, `shared.metadata` は merge というより探索パスとして扱う。

実行時の探索順:

1. 巻固有パス
2. `shared/*`

これは config merge ではなく、resource resolution の規則として扱う。

## 8. path 正規化

### 8.1 config 入力

config に現れるパスはすべて `RepoPath` として扱う。

ルール:

- `/` 区切り
- repo root 基準
- 絶対パス禁止
- `..` を含む場合は repo 外へ出ないこと

### 8.2 実行時変換

`RepoPath` は repo root と結合して `FsPath` に変換する。

方針:

- config 内表現は常に `/` のまま保持
- 実ファイルアクセス時のみ OS 依存パスへ変換
- エラーメッセージには可能な限り `RepoPath` を出す

## 9. resource 解決

### 9.1 `single-book`

探索順:

1. `assets/`
2. `styles/`
3. 参照元ファイルと同階層

### 9.2 `series`

探索順:

1. `books/<book-id>/assets/`
2. `books/<book-id>/styles/`
3. `shared/assets/`
4. `shared/styles/`
5. 参照元ファイルと同階層

## 10. 代表的な解決例

### 10.1 `single-book`

```text
repo/
  book.yml
  manuscript/01.md
```

`repo/manuscript` で `shosei build` を実行した場合:

- repo root = `repo/`
- mode = `single-book`
- target book = `repo/book.yml`

### 10.2 `series`

```text
repo/
  series.yml
  books/vol-01/book.yml
```

`repo/books/vol-01` で `shosei build` を実行した場合:

- repo root = `repo/`
- mode = `series`
- target book = `books/vol-01/book.yml`

### 10.3 `series` root で巻未指定

`repo/` で `shosei build` を実行した場合:

- repo root = `repo/`
- mode = `series`
- target book = unresolved
- 結果 = `--book` を要求する error

## 11. エラー条件

- `book.yml` と `series.yml` が同一ディレクトリに共存
- `series.yml` で指定された `books[].path` に `book.yml` が存在しない
- `--book` 指定 ID が `series.yml` に存在しない
- config path が repo 外を指す
- merge 後に required field が欠落

## 12. v0.2 の決定

- repo root は上方探索で見つける
- `series` では `--book` またはカレントパスから対象巻を決める
- 設定優先順位は `CLI > book.yml > series.yml > profile defaults`
- array は基本置換
- `shared/*` は merge 対象ではなく探索パスとして扱う
- `manuscript.chapters` の順序は保持し、filename prefix から並び替えない

## 13. `shosei explain` での由来表示

`shosei explain` は、解決済みの設定値に対して少なくとも次の由来カテゴリを表示する。

- `book.yml`
- `series defaults`
- `built-in default`

補足:

- `shared.assets`, `shared.styles`, `shared.fonts`, `shared.metadata` は merge 値ではなく、`series.yml` の探索パスとして別表示する
- v0.2 の `explain` は text 出力を基本とし、すべての schema 項目を網羅しなくてもよい
- editor integration では `shosei explain --json` を使って同じ resolved config を機械可読で取得してよい
- 初期化済みの `reference/` と `story/` workspace があれば、`explain` の text summary と `--json` snapshot に current scope / shared scope の file 概要を含めてよい
- book-scoped な `story/scene-notes/` と `story/structures/` があれば、`explain` はその file 概要も story workspace の一部として含めてよい
- prose / manga で relevant な項目だけを表示してよい
