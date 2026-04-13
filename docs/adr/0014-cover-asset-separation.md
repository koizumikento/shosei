# ADR-0014: 外部カバー画像は `book.yml` で明示し、本文ページと分離する

- Status: Accepted
- Date: 2026-04-13

## Context

現状の仕様には次が同居している。

- `assets/cover/` というディレクトリ
- `manuscript/00-cover.md` という prose 側の例
- `print.cover_pdf` という印刷向けフラグ

一方で、`book.yml` には cover 専用 schema が未定義で、Kindle/EPUB の外部カバー画像と本文フロー内のカバーページが混同されやすい。

このままだと、build/validate/handoff がどのファイルを cover asset として扱うべきかを推測に頼ることになる。

## Decision

v0.1 の `book.yml` に `cover` セクションを追加し、外部カバー画像は明示 path で指定する。

最小形:

```yaml
cover:
  ebook_image: assets/cover/front.jpg
```

ルール:

- `cover.ebook_image` は repo root 基準の相対パス
- v0.1 では `.jpg`, `.jpeg`, `.png` を許可する
- `cover.ebook_image` は Kindle/EPUB 向けの外部カバー画像を表す
- `manuscript` や `sections.type: cover` は本文フロー内ページを表し、`cover.ebook_image` の代替にはしない
- `cover` は巻固有情報として扱い、`series.yml` の `defaults` では継承対象にしない
- 印刷カバーの source 定義は別課題として扱い、`print.cover_pdf` の詳細化時に決める

## Consequences

- build/validate/handoff は cover asset を推測ではなく明示設定から取得できる
- `assets/cover/` を標準ディレクトリとして維持しやすい
- `00-cover.md` のような本文ファイルは扉や本文内カバーページとして扱える
- Kindle/EPUB cover と print cover の責務境界が明確になる
- 将来は print cover source、複数 cover variant、出力 target ごとの cover override を別途拡張できる
