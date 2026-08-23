# ADR-0012: Resource finalizationを独立phaseにする

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

Parser後・shaping前の下位`typaxis-resource-admission` crateが、HostAdmissionContext-bound sealed root setから発行されたsame-handle source receiptだけを読み、resource bytes/hash/bytes-derived metadataを固定する。pending bytes/metadata receiptはbudgetをconsumeしたresolver sessionにもbindし、foreign resolverへの移送を拒否する。ResourceCollectorはrepeat useをlogical IDごとにunionし、selected Display LayoutEpochのadmitted fingerprintと供給ledgerをexact照合し、同じIDが異なるadmitted hash/metadataへ解決すればerrorにする。Display ListまではPDF非依存とし、late finalizer以降をprofile 1.0のPDF-specific phaseとする。late finalizerはfontを`(font, admitted source SHA-256, FontInstanceId)`、imageを`(image, admitted source SHA-256, ImageResourceId)`でcanonical sortし、dedupe後のduplicate keyを拒否してsubset/CID/extraction/descriptor metrics/image encoding/typed indirect-object blueprintのPDF-readyかつbackend-identity-freeな`FrozenPdfResourcePlans`を確定する。subsetter receiptはembedded `name` tableから再抽出したdeterministic subset PostScript名をbindし、finalizerはFontInstanceId由来の期待名とexact照合する。backend handle、PDF resource name、object IDは持たず、それらはfrozen planを受けるPDF backendだけが割り当てる。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
