# ADR-0022: story sync は explicit のまま 1 entity の双方向 copy を許可する

- Status: Accepted
- Date: 2026-04-14

## Context

`story sync` は最初、shared canon から巻固有 story workspace への one-way copy として導入した。

ただし運用上は、巻側で整理した entity を shared canon に昇格させたい場面もある。毎回手動 copy に戻すと、`story drift` で見つけた差分の修復フローが片道だけになってしまう。

一方で、automatic merge や双方向の暗黙同期まで入れると、優先順位と conflict 解消方針を過剰に固定する。

## Decision

`story sync` は explicit な 1 entity copy のまま、`--from shared` と `--to shared` の両方向を許可する。

ルール:

- 対象は `series` のみ
- `--from shared` か `--to shared` のどちらか一方だけを受け付ける
- 対象 entity は `kind` と `id` で 1 件ずつ指定する
- destination 側に同じ `id` が無い場合は copy する
- destination 側に同じ `id` があり内容も同じ場合は no-op とする
- destination 側に同じ `id` があり内容が違う場合は、`--force` が無い限り上書きしない
- `scenes.yml` の更新や automatic merge は行わない

## Consequences

- `story drift` の後に、shared→book と book→shared の両方で最小の修復操作を取れる
- 書き込み方向は毎回明示されるので、暗黙の precedence を増やさずに済む
- 将来の bulk sync や merge 戦略は、別の意思決定として切り出しやすい
