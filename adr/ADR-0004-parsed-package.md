# ADR-0004: Parserは完全なParsedPackageを返す

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

sources、text store、document、styles、page masters、resourcesを所有権の異なるfieldとして一つの解析結果にする。syntax crate内のsealed source-driven Parserがcross-source validationを完了した`ValidatedParsedPackage`だけを`ParseOutcome::Parsed`で返し、caller-built `ParsedPackage`やfixture featureからtrusted resultへ昇格する経路を持たない。diagnosticsはnote/warningだけの`AdvisoryDiagnostic`に限定する。`ParseOutcome::Failed`は少なくとも1件のerrorまたはfatalを持ちpackageを持たない。fatalは即時打切り、errorは安全なparser境界まで収集できるが、どちらもpackage valueやartifact successと同時に表現できない。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
