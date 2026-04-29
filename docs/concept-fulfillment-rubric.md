# Concept Fulfillment Rubric

この文書は、`shosei` の「コンセプト充足度チェック」を毎回同じ基準で行うための固定 rubric です。

以後の recurring review では、この rubric を採点基準の source of truth とし、別の重みや別解釈をその場で作らないこと。

## Goal

評価対象は「`shosei` が、この repo のコンセプトどおりの製品としてどこまで実装・証明・説明できているか」です。

これは雰囲気評価ではなく、次の 3 つをまとめて見る。

1. 実装があるか
2. その実装が継続的に証明されているか
3. 公開 docs がその実態を正しく説明しているか

## Fixed Scale

- 総合点は `0.0` から `10.0`
- 採点単位は `0.5` 刻み
- 毎回、下の 5 項目を同じ重みで採点する
- 総合点は 5 項目の合計

## Scoring Categories

### 1. Core workflow coverage: 0.0 - 3.0

対象:

- `init`
- `explain`
- `build`
- `validate`
- `preview`
- `doctor`
- `handoff`
- `series sync`
- `page check`
- `reference`
- `story`

目安:

- `3.0`: 主要 workflow が user-facing surface として実装済み
- `2.0`: 主要 workflow は揃うが、穴や限定条件がまだ目立つ
- `1.0`: surface はあるが、実体が薄い
- `0.0`: 構想段階

### 2. Delivery-grade validation and packaging: 0.0 - 2.5

対象:

- `validate` report quality
- target/profile ごとの warning / validator
- `handoff` manifest / review packet / package contents

目安:

- `2.5`: validate と handoff が delivery 前提でかなり信頼できる
- `1.5`: local lint と package 導線はあるが、外部 validator や target-specific depth はまだ薄い
- `0.5`: 枠組みだけ
- `0.0`: 未着手

解釈:

- この項目の `validator` は、現在の product concept と specs が delivery 前提として扱う validator / evidence を指す。
- specs / docs で明示的に advisory future work として分離され、handoff blocker ではないと説明されている validator は、それ自体を未実装減点に使わない。
- 例: Kindle Previewer 以外の store / device 固有 validator は、`unsupported_checks[]` に advisory として正しく記録・説明されていれば減点対象ではない。
- 減点対象になるのは、現在の concept 上必要な validator / evidence が薄い場合、または advisory / unsupported の扱いが docs や report で誤解を招く場合。

### 3. Cross-platform and proof in CI: 0.0 - 2.0

対象:

- macOS / Windows / Linux での command surface の証明
- named smoke / test の見え方
- VS Code adapter の継続検証

目安:

- `2.0`: 主要 surface が CI で継続証明され、OS ごとの保証が読める
- `1.0`: 一部は証明されているが、見える保証がまだ狭い
- `0.5`: CI はあるが製品保証として弱い
- `0.0`: 未整備

### 4. Docs truthfulness and sync: 0.0 - 1.5

対象:

- `README.md`
- `docs/usage.md`
- `site/usage.html`
- `site/index.html`
- install / release docs
- VS Code README / spec sync

目安:

- `1.5`: docs が current implementation と揃っている
- `1.0`: 概ね揃うが、一部 stale な説明が残る
- `0.5`: 重要な食い違いがある
- `0.0`: docs が現状を表していない

### 5. Product readiness and coherence: 0.0 - 1.0

対象:

- surface 間の一貫性
- 「立ち上げ段階」からどこまで製品として読めるか
- README / site / CI / release の一体感

目安:

- `1.0`: まだ成長途上でも、製品としてのまとまりがある
- `0.5`: 実装はあるが、見え方がまだ部分最適
- `0.0`: 断片的

## Required Review Procedure

毎回の concept-fulfillment review では、次を守ること。

1. 5 項目すべてに subscore を付ける
2. 各 subscore に、少なくとも 1 つは file evidence を付ける
3. 総合点は subscore の合計だけで決める
4. 最後に前回との差分を明示する

## Non-regression Rule

点数は、次のどちらかがある場合にだけ上下させてよい。

1. 実装・CI・docs の証拠が変わった
2. 前回の採点がこの rubric に照らして明確に誤っていた

次の場合は減点しない。

- 以前から分かっていた未実装項目を、別カテゴリで二重に数える
- 今回たまたま読む範囲が広がっただけで、新しい事実が増えていない
- docs の表現差を、実装差のように扱う
- specs / docs で advisory future work として明示された範囲外 validator が、未実装のまま正直に `unsupported` / advisory として扱われている

## Delta Rule

前回と同じ証拠水準なら、総合点も維持する。

点数変更時は、必ず次の形で差分理由を書く。

- `score delta`
- `which category changed`
- `new evidence`
- `why the previous score is no longer correct`

## Output Format

毎回の出力は最低でも次を含める。

```text
Core workflow coverage: X / 3.0
Delivery-grade validation and packaging: X / 2.5
Cross-platform and proof in CI: X / 2.0
Docs truthfulness and sync: X / 1.5
Product readiness and coherence: X / 1.0
Overall: X / 10.0
Delta from previous run: ...
```

## Interpretation Bands

- `9.0 - 10.0`: concept is strongly fulfilled
- `8.0 - 8.5`: good, with limited structural gaps
- `7.0 - 7.5`: broadly implemented but still missing product-level proof or consistency
- `5.0 - 6.5`: partial fulfillment
- `< 5.0`: concept is still mostly aspirational
