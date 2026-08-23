# ADR-0016: 再現性入力をbuild manifestへ記録する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

data table、shaper、font/image bytes、config、output hashを追跡する。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
