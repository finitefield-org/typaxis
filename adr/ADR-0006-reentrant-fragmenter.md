# ADR-0006: Paginationは再入可能Fragmenterを使う

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

footnote/frame変更後に同じcursorから本文を再フローできるようにする。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
