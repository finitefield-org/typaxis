# ADR-0014: 初期PDF profileを1.7 classic xref生成に限定する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

広いviewer互換性と実装監査容易性を優先する。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
