# ADR-0003: SourceSpanとTextSpanを分離する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

SourceCatalogはadmitted source bytes/path/hash、parsed TextStoreはUnicode buffer/text mapを別所有し、ParsedPackageが別fieldとして束ねる。SourceIdとTextBufferIdを共有しない。正規化・escape展開・syntax-time inserted textをparsed TextStoreとsource mapで追跡し、identity mapは対応rangeのbyte lengthとbytesが完全一致する場合だけ許可する。state-dependent textはLayoutPassCoordinator所有のimmutable GeneratedTextStoreへ置く。allocation-independentなGeneratedBufferKeyをsortして別GeneratedTextBufferIdをdense割当し、Display境界でparsed/generated bufferをTextBufferId/GeneratedBufferKey順のdense DisplayTextBufferIdへremapする。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
