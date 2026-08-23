# ADR-0010: Unicode所有単位をglyphではなくclusterにする

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

ToUnicode重複を避け、必要時にActualTextをclusterへ適用する。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
