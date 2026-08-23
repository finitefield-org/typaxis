# ADR-0008: PDF非依存Display Listを置く

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

layoutとbackendを分離しlogical IDとtyped geometryだけを渡す。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
