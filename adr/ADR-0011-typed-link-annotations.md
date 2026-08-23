# ADR-0011: Linkをtyped page annotationにする

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

graphics stateとannotationを分離しraw PDF actionを禁止する。URI targetはsyntax境界で検証済み`SafeUri`だけを受け、annotation/destination座標はcontent CTM外でpage heightを使ってPDF user spaceへ変換する。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
