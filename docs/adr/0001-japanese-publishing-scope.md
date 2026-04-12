# ADR-0001: 日本語出版向け CLI を主対象にする

- Status: Accepted
- Date: 2026-04-12

## Context

当初は EPUB/PDF をまとめて扱う電子書籍コンパイル CLI を想定していたが、議論を進めると要件の重心は日本語出版の実務にあった。

具体的には以下が重かった。

- Kindle 日本語向けの提出
- 日本の印刷会社への入稿
- 縦書き/横書き
- 小説、ライトノベル、漫画といった日本語書籍カテゴリ

## Decision

本ツールは、汎用 document converter ではなく、日本語出版向け制作 CLI を主対象とする。

優先ターゲットは以下とする。

- Kindle 日本語向け EPUB
- 日本の印刷会社向け PDF

## Consequences

- 海外向け一般 PDF/EPUB 最適化は後回しになる
- target profile が日本市場の慣習を前提にした設計になる
- UI/用語も出版実務寄りに設計する必要がある
