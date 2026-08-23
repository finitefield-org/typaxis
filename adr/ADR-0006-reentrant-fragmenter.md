# ADR-0006: Paginationは再入可能Fragmenterを使う

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

footnote/frame/reference state変更後にglobal structured `FlowPosition`から本文を再フローできるようにする。`Continuation::More`は同じepoch内でpositionを厳密に前進させ、owner境界を跨げる。opaque fingerprintの変化だけを進捗とみなさない。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
