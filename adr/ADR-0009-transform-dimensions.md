# ADR-0009: Affine transformの次元を型で分離する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

linear termsは16.16 unitless、translationはLengthとする。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
