# ADR-0018: prose 向け editorial metadata は sidecar file で扱い、`handoff proof` に review packet を含める

- Status: Accepted
- Date: 2026-04-13

## Context

技術書やビジネス書では、本文に加えて次の管理が必要になる。

- 表記ゆれや禁止語の統制
- 図表の caption、出典、権利メモ
- 主張ごとの根拠メモ
- 鮮度確認が必要な箇所の再確認期限
- 外部校正や編集者に渡す review packet

一方で、これらを `book.yml` に直接増やすと、build 用設定と editorial 運用情報が混ざりやすい。

また、`shosei` は WYSIWYG や共同編集 UI ではなく、`validate` と `handoff` を通じた制作フロー制御を近接価値に置いている。

## Decision

prose 系 project では、editorial metadata を `book.yml` / `series.yml` から参照する sidecar file として扱う。

`book.yml` には参照先だけを持たせる。

```yaml
editorial:
  style: editorial/style.yml
  claims: editorial/claims.yml
  figures: editorial/figures.yml
  freshness: editorial/freshness.yml
```

sidecar の責務は次の通りとする。

- `style.yml`
  - 推奨表記
  - 禁止語
- `claims.yml`
  - claim id
  - 要約
  - 対応 section
  - source 一覧
  - reviewer note
- `figures.yml`
  - figure id
  - asset path
  - caption
  - source
  - rights
  - reviewer note
- `freshness.yml`
  - claim / figure ごとの `last_verified`
  - `review_due_on`
  - 再確認メモ

`shosei validate` は prose 系で次を検査する。

- style sidecar に基づく推奨表記と禁止語
- figure ledger の asset / source / manuscript 参照整合
- claim ledger の section / source 整合
- freshness ledger の参照整合と review due

`shosei explain` は editorial sidecar の参照先と件数を表示する。

`shosei handoff proof` は review packet を生成し、少なくとも次を含める。

- 校正用成果物
- validate report
- editorial sidecar のコピー
- unresolved issue と reviewer note をまとめた human-readable な review note
- unresolved issue、reviewer note、claim / figure / freshness を列挙した machine-readable な review packet JSON
- `manifest.json` の `review_notes` / `review_packet` 参照
- `manifest.json` の `editorial_summary.claim_count` / `figure_count`
- `reports/review-packet.json` の `issue_summary` / `reviewer_notes` / `editorial_summary`

## Consequences

- `book.yml` は build 設定中心のまま保てる
- editorial metadata は prose project で段階的に導入しやすい
- `validate` の守備範囲が、本文 lint から review readiness まで広がる
- `handoff proof` は単なる成果物コピーではなく、校正・編集の受け渡し単位になる
- series で editorial defaults を共通化したい場合は、将来別の merge ルールを追加する
