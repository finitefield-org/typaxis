# ADR-0009: Affine transformの次元を型で分離する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

linear termsは16.16 unitless、translationはLengthとする。column vectorへ`x'=a*x+c*y+e, y'=b*x+d*y+f`を適用し、concatは`CTM := CTM * M`とする。page rootは高さHに対して`(1,0,0,-1,0,H)`とする。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
