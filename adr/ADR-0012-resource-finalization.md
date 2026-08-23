# ADR-0012: Resource finalizationを独立phaseにする

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

stable sort後にsubset/CID/backend handle/PDF name/object IDを確定する。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
