# ADR-0015: Canonical IRへnamespaced exact contract IDを使う

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

unknown contractを解釈せず、additionalProperties falseと整合させる。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
