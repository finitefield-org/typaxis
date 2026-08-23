# ADR-0001: Semantic ASTをPDFから分離する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

Documentは意味構造とlogical resource referenceだけを持ち、座標、glyph、CID、PDF name/object IDを持たない。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
