# ADR-0016: 再現性入力をbuild manifestへ記録する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

data table、shaper、font/image bytes、effective config、PDF stream compression mode、layout status/pass count/selected state/final fingerprint/fallback policy、output hashを追跡する。effective configはdefaults、file、environment、CLIの順に解決し、set意味のresource roots/URI schemesをunique UTF-8-byte-sorted arrayへ正規化してcontract-defined JSON valueのRFC 8785 JCS bytesをhashする。manifest record arraysはcontract-defined unique canonical orderにし、fallback score詳細はtraceだけに置く。

manifest optionの有無にかかわらず、CLI/target/config解決後にnon-cloneable per-build `BuildOutputCommitContext`を作る。manifest target指定時だけsame-sessionのnon-cloneable `ManifestPublicationContext`を追加し、target未指定buildのPDF単独commit経路を残す一方、指定済みsessionではその経路を拒否する。canonical statusはterminalなbuilt/failedだけとする。failed output-null recordはpublication-owned admission factsからsealed preflightし、atomic sidecar commit後だけtrusted manifest/receiptを公開する。layoutを持つfailed recordはvalidated package epochとadmitted resource fingerprintの完全なidentityを要求する。

built outputは1 page以上で、host path/URIではなくfile/stdout `OutputSink`を記録する。package/resource/pagination/non-cloneable verified PDF bytesの全closure、compression、limits、canonical manifest bytesをI/O前にpreflightする。self-consuming terminal APIだけがexact output sessionのtokenを受け、PDFをcomplete commitした後にmanifestをsame-directory temp write/fsync/atomic publishし、両方の成功後だけtrusted manifest、PDF sink receipt、manifest sink receiptを公開する。別session token、clone、caller record、success callbackではpublicationできない。manifest I/O failureは既にcommit/emission済みのPDF receiptをerrorに保持し、rollbackしたとは扱わない。output/trace/manifest target identityはsession構築時、write開始時、tempのfinal publish直前に再解決してaliasをfail closedにする。profile 1.0は常にdeterministicとする。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
- reference workspaceでplatform-native atomic primitiveが登録されないplatform/filesystemはreceiptを発行せずfail closedにする。
