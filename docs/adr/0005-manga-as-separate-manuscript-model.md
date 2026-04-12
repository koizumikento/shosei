# ADR-0005: 漫画を別原稿モデルとして扱う

- Status: Accepted
- Date: 2026-04-12

## Context

当初はビジネス書、小説、ライトノベルを中心に考えていたが、その後で漫画カテゴリも対象にしたいという要件が追加された。

漫画は文章主体の原稿とは異なり、ページ画像、見開き、左右ページ、カラーページ、固定レイアウト成果物などを中心に扱う。

## Decision

`manga` は prose 系とは別の原稿モデルと build パイプラインを持つ。

要素:

- volume
- chapter
- page
- spread
- page image assets

将来的には制作工程も管理対象に含める。

- script
- storyboard
- art
- export
- validate
- handoff

## Consequences

- Pandoc 中心の prose build と漫画 build は分離される
- 同じ CLI の下で、複数原稿モデルを扱う設計が必要になる
- fixed-layout や画像検証機能の比重が上がる
