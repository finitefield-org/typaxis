# ADR-0002: 長さを1/65536 PDF pointにする

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

TeX pointと区別し、exact rational conversionとround-half-to-evenを採用する。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
