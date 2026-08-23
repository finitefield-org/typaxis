# ADR-0005: 段落をBox/Glue/Penaltyへ正規化する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

greedyとoptimalを同一input modelで差し替える。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
