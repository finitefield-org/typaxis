# ADR-0013: Streamを間接objectに限定しgraphをfreezeする

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

Lengthとreference integrityをserializer前に固定する。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
