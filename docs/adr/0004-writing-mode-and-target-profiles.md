# ADR-0004: 縦書き・横書きと target profile を原稿モデルに組み込む

- Status: Accepted
- Date: 2026-04-12

## Context

議論の中で、縦書き/横書き対応、Kindle、日本の印刷会社向け PDF、ライトノベルの画像配置など、出力先と writing mode が強く結びついていることが明らかになった。

これを単なる見た目のオプションとして扱うと、プロジェクト全体の整合性を保ちにくい。

## Decision

`writing_mode`, `reading_direction`, `binding`, `target profile` は原稿モデルの一部として扱う。

代表的な target profile:

- `kindle-ja`
- `print-jp-pdfx1a`
- `print-jp-pdfx4`
- `kindle-comic`
- `print-manga`

## Consequences

- section type や画像配置も profile と連動して設計する必要がある
- Kindle と印刷の差分を profile で吸収できる
- prose と manga の build graph を分けやすくなる
