# ADR-0007: 収束をinput/output fingerprint chainで判定する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

stable、cycle、max-passを区別して必ず停止する。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
