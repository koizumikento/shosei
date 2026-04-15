# ADR-0027: prose の default design は scaffolded stylesheet で持ち、build 時に適用する

- Status: Accepted
- Date: 2026-04-14

## Context

`shosei init` は prose 系 scaffold として `styles/base.css`, `styles/epub.css`, `styles/print.css` を生成してきた。

一方で、v0.1 の実装には次のズレがあった。

- `single-book` では 3 つの stylesheet を作るが、`series` では `shared/styles/base.css` しか作らない
- prose EPUB build は `epub.css` を scaffold していても Pandoc に渡していない
- `book.writing_mode` と `layout.binding` は config にあるが、default style 側では template/profile ごとの差が薄く、出力の見た目から追いにくい

この状態だと、template/profile ごとの default design が「存在するように見えるが、実際には一部しか効いていない」状態になる。

## Decision

prose の default design は template/profile が所有し、scaffolded stylesheet と build wiring の組で表現する。

具体的には次の方針とする。

- prose 系 scaffold は `single-book` / `series` の両方で `base.css`, `epub.css`, `print.css` を style root に生成する
  - `single-book`: `styles/`
  - `series`: `shared/styles/`
- prose EPUB build は `base.css` と `epub.css` を Pandoc に渡す
- prose print build は従来どおり `base.css`, `print.css`, generated layout stylesheet を合わせて渡す
- default style の責務は template/profile ごとの読みやすさと初期見た目を与えることに留める
  - `business`, `paper`: 横組み prose の既定
  - `novel`, `light-novel`: 縦組み prose の既定
    - `base.css` で組方向と本文の基本 rhythm を持つ
    - `print.css` で PDF 向けの本文サイズと frontmatter の見た目を整える
    - build-generated print stylesheet は vertical prose print の frontmatter pagination を持つ
      - TOC がある場合は title と TOC を同じ前付けに保ったまま、本文だけを次ページに送る
      - TOC が無効な場合は title の後で本文へ入る前に改ページする
  - `conference-preprint`: `paper` 系 style を継承しつつ、2 段組や余白などの強い layout 差分は config-generated print stylesheet で表す
- `manga` は fixed-layout EPUB を別原稿モデルとして扱い、見た目の source of truth は引き続き manga build pipeline 内の CSS と metadata に置く

## Consequences

- `init` scaffold はこれまでより少し意図を持った CSS を出力する
- `series` の style surface が `single-book` に近づき、docs と editor integration の説明を揃えやすくなる
- prose の `writing_mode` 差分が EPUB / print の default 出力にも反映されやすくなる
- prose の PDF 既定タイポグラフィ調整は `print.css` に閉じ込められ、Kindle / EPUB の読み味と分離して扱える
- default design を変更するときは、style scaffold だけでなく build wiring と usage docs も同時に確認する必要がある
