# ADR-0004: Parserは完全なParsedPackageを返す

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

sources、text store、document、styles、page masters、resourcesを不可分の解析結果にする。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
