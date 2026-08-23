# ADR-0013: Streamを間接objectに限定しgraphをfreezeする

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

stream payload、filter policy、reference integrityをserializer前にfreezeする。`Length`、`Filter`、`DecodeParms`のdictionary materializationはserializerが所有し、filter適用後bytesからLengthを生成する。untrusted direct valueとPages hierarchyはroot containerをdepth 1とするiterative inclusive depth-64 validationを使い、recursive `PdfValue` childrenもiterativeにdropする。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
