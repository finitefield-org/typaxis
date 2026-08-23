# ADR-0003: SourceSpanとTextSpanを分離する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

正規化・escape展開・generated textをTextStoreとsource mapで追跡する。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
