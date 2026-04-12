# `single-book` から `series` への移行仕様 v0.1

作成日: 2026-04-12  
状態: Draft

## 1. 目的

この文書は、`single-book` で始めたリポジトリを、後から `series` に昇格させる手順を定義する。

対象:

- 既存 `book.yml` の移動
- `series.yml` の新規生成
- ディレクトリ再編成
- 相対パスの維持
- Git 履歴を壊さない移行

## 2. 背景

実運用では、最初は単巻完結や単発作品として始めても、後から続刊化することがある。

特に次のケースでは移行需要が高い。

- 小説がシリーズ化する
- ライトノベルの続刊が決まる
- 読切漫画が連載化する
- 共通 assets/styles を巻横断で持ちたくなる

## 3. 結論

v0.1 では、`single-book -> series` の移行を正式に考慮する。

推奨する移行手段:

```bash
shosei migrate --to series --book-id vol-01
```

このコマンドは v0.1 では将来候補だが、仕様は先に固定する。

## 4. 対象外

- `series -> single-book` の逆変換
- 1 repo 内に複数シリーズを作る変換
- `multi-series` への変換
- Git 履歴の rewrite

## 5. 移行前提条件

対象 repo は次を満たすこと。

- root に `book.yml` がある
- root に `series.yml` がない
- `single-book` として解決できる
- `book.yml` の必須項目が妥当

推奨条件:

- clean worktree
- 直前に commit 済み

## 6. 移行後の目標構造

```text
repo/
  series.yml
  shared/
    assets/
    styles/
    fonts/
    metadata/
  books/
    vol-01/
      book.yml
      manuscript/
      manga/
      assets/
  dist/
  .gitignore
  .gitattributes
```

## 7. 基本移行ルール

### 7.1 設定ファイル

- 既存 root `book.yml` は `books/<book-id>/book.yml` へ移動する
- 新規に root `series.yml` を生成する

### 7.2 ディレクトリ

移動候補:

- `manuscript/` -> `books/<book-id>/manuscript/`
- `manga/` -> `books/<book-id>/manga/`
- `assets/` -> `books/<book-id>/assets/`

共有候補:

- `styles/` -> `shared/styles/`
- `assets/fonts/` -> `shared/fonts/`

### 7.3 パス

- config 内の path は repo root 基準のまま再書換えする
- `manuscript/...` は `books/<book-id>/manuscript/...` に変換する
- `assets/...` は移動先に応じて `books/<book-id>/assets/...` または `shared/...` に変換する

## 8. 生成される `series.yml`

最小限、次を含む。

```yaml
series:
  id: my-series
  title: "シリーズ名"
  language: ja
  type: novel

defaults:
  book:
    profile: novel
    writing_mode: vertical-rl
    reading_direction: rtl
  layout:
    binding: right

git:
  lfs: true
  require_clean_worktree_for_handoff: true

books:
  - id: vol-01
    path: books/vol-01
    number: 1
    title: "第1巻"
```

生成方針:

- 既存 `book.yml` から共通化しやすい値を `defaults` に上げる
- 巻固有の値は `books/<book-id>/book.yml` に残す

## 9. `book.yml` の再書換えルール

### 9.1 `series.yml` に上げる候補

- `git`
- `validation`
- `book.profile`
- `book.writing_mode`
- `book.reading_direction`
- `layout.binding`
- `outputs` の既定 target
- `pdf` の共通設定
- `print` の共通設定
- `images` の共通設定

### 9.2 巻側に残すもの

- `book.title`
- `book.subtitle`
- `book.identifier`
- `manuscript`
- `sections`
- 巻固有 `assets`
- 巻固有の挿絵・ページ画像

### 9.3 v0.1 の保守的方針

自動移行時は、過度な共通化をしない。

- 共通化しやすい明白な項目だけ `series.yml` に上げる
- 判断が割れる項目は巻側に残す

理由:

- 間違った共通化より、少し冗長でも安全な移行を優先する

## 10. ディレクトリ移動方針

### 10.1 prose

```text
before:
  book.yml
  manuscript/
  assets/
  styles/

after:
  series.yml
  shared/styles/
  books/vol-01/book.yml
  books/vol-01/manuscript/
  books/vol-01/assets/
```

### 10.2 manga

```text
before:
  book.yml
  manga/
  assets/
  styles/

after:
  series.yml
  shared/styles/
  shared/fonts/
  books/vol-01/book.yml
  books/vol-01/manga/
  books/vol-01/assets/
```

### 10.3 共通化の既定

- `styles/` は `shared/styles/` に移す
- `assets/fonts/` は `shared/fonts/` に移す
- `assets/images/` は既定では巻側に残す

## 11. Git の扱い

移行は rename ベースで行う。

方針:

- 可能な限り `git mv` 相当の移動を使う
- Git 履歴は rewrite しない
- 移行前に clean worktree を推奨
- dirty worktree では warning、`--strict-clean` では error

## 12. CLI 仕様案

### 基本形

```bash
shosei migrate --to series --book-id vol-01
```

### オプション候補

- `--series-id`
- `--series-title`
- `--book-id`
- `--book-number`
- `--dry-run`
- `--strict-clean`
- `--move-styles-to-shared`
- `--move-fonts-to-shared`

### 挙動

- `--dry-run` では移動計画だけ表示
- デフォルトでは安全な rename と config 再書換えのみ行う

## 13. 実行フロー

1. repo mode を `single-book` と確認
2. `book.yml` を読む
3. 移行計画を生成
4. 衝突チェック
5. `series.yml` を生成
6. ディレクトリ rename
7. `books/<book-id>/book.yml` を再書換え
8. summary を表示

## 14. 衝突チェック

error にする条件:

- 既に `series.yml` がある
- `books/<book-id>/` が既に存在する
- 目的地に同名ファイルがある
- 必須 metadata が足りず `series.yml` を生成できない

warning にする条件:

- dirty worktree
- `styles/` や `fonts/` の共通化が曖昧
- root に巻固有画像が大量にあり、自動分類できない

## 15. diagnostics 出力

移行結果として、最低限次を表示する。

- 新しい repo mode
- 生成した `series.yml`
- 作成した `books/<book-id>/`
- moved files
- shared に移した assets
- 手動確認が必要な項目

## 16. 手動確認チェックリスト

- `books/<book-id>/book.yml` の path が正しいか
- `series.yml` の `defaults` に上げ過ぎていないか
- `shared/styles/` の参照が正しいか
- build / validate が通るか

## 17. v0.1 の決定

- `single-book -> series` は将来機能だが、仕様として先に固定する
- 移行は安全優先
- 既定では 1 冊目を `vol-01` として移す
- 自動共通化は保守的に行う
