# ADR-0005: 段落をBox/Glue/Penaltyへ正規化する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

greedyとoptimalを同一immutable input modelで差し替える。text-bearing itemはshaped run slice、BidiLevel、parsed TextSpanまたはallocation-independentなGeneratedBufferKeyとGeneratedTextSpanを含むepoch-uniqueな完全`GeneratedProvenance`を保持し、Discretionaryはno-break/pre-break/post-break各branchの描画contentを明示する。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
