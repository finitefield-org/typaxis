# ADR-0011: Linkをtyped page annotationにする

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

graphics stateとannotationを分離しraw PDF actionを禁止する。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
