# ADR catalog

この一覧は現行contractに適用する判断と、明示的にfuture targetとして採択した判断を責務ごとに整理したものです。各ADRの`Status`とimplementation statusを確認し、Accepted targetを現行CLI/Schema/releaseで利用可能という意味に読み替えません。

## Core model と portability

- [ADR-0001: Semantic ASTをPDFから分離する](ADR-0001-semantic-ast.md)
- [ADR-0002: 長さを1/65536 PDF pointにする](ADR-0002-pdf-point-fixed-unit.md)
- [ADR-0003: SourceSpanとTextSpanを分離する](ADR-0003-source-text-separation.md)
- [ADR-0004: Parserは完全なParsedPackageを返す](ADR-0004-parsed-package.md)
- [ADR-0009: Affine transformの次元を型で分離する](ADR-0009-transform-dimensions.md)
- [ADR-0015: Canonical IRへnamespaced exact contract IDを使う](ADR-0015-contract-versioning.md)
- [ADR-0017: Local text-map ranges](ADR-0017-local-text-map-range.md)
- [ADR-0018: Portable contained paths](ADR-0018-portable-contained-paths.md)

## Text、style、layout

- [ADR-0005: 段落をBox/Glue/Penaltyへ正規化する](ADR-0005-box-glue-penalty.md)
- [ADR-0006: Paginationは再入可能Fragmenterを使う](ADR-0006-reentrant-fragmenter.md)
- [ADR-0007: 収束をinput/output fingerprint chainで判定する](ADR-0007-bounded-convergence.md)
- [ADR-0010: Unicode所有単位をglyphではなくclusterにする](ADR-0010-cluster-extraction.md)
- [ADR-0019: State-indexed pagination](ADR-0019-state-indexed-pagination.md)
- [ADR-0021: Bidi and paragraph-item IR](ADR-0021-bidi-and-paragraph-items.md)
- [ADR-0025: Block selector and inheritance cascade](ADR-0025-block-selector-and-inheritance-cascade.md)
- [ADR-0026: Page selection context and PageName](ADR-0026-page-selection-context.md)

## Display、resource、PDF

- [ADR-0008: PDF非依存Display Listを置く](ADR-0008-display-list.md)
- [ADR-0011: Linkをtyped page annotationにする](ADR-0011-typed-link-annotations.md)
- [ADR-0012: Resource finalizationを独立phaseにする](ADR-0012-resource-finalization.md)
- [ADR-0013: Streamを間接objectに限定しgraphをfreezeする](ADR-0013-pdf-stream-and-freeze.md)
- [ADR-0014: 初期PDF profileを1.7 classic xref生成に限定する](ADR-0014-pdf-generation-profile.md)
- [ADR-0020: Display destinations and paint](ADR-0020-display-destinations-and-paint.md)
- [ADR-0022: PDF-ready resource plans](ADR-0022-pdf-ready-resource-plans.md)
- [ADR-0023: Stream ownership and page-tree validation](ADR-0023-stream-and-page-tree-validation.md)

## Build と配布

- [ADR-0016: 再現性入力をbuild manifestへ記録する](ADR-0016-build-manifest.md)
- [ADR-0024: Stored reproducible release archive](ADR-0024-stored-release-archive.md)

## Machine input target

- [ADR-0027: Machine DocumentPackage ingestion and immutable PDF profile](ADR-0027-machine-document-package-ingestion.md) — M1 targetとしてAccepted・実装済み。contract 1.1、public CLI E2E、macOS/Linux evidenceをMI1-17で公開済み。
- [ADR-0028: Basic document machine-PDF profile](ADR-0028-basic-document-profile.md) — M2 targetとしてAccepted。contract 1.2と`basic-document-1`はMI2-08で公開済み。
- [ADR-0029: Table machine-PDF profile](ADR-0029-table-profile.md) — M3 table targetとしてAccepted。current contract 1.2を変更せず、`table-1`はMI3-04で公開済み。
- [ADR-0030: Footnote machine-PDF profile](ADR-0030-footnote-profile.md) — M3 footnote targetとしてAccepted。current contract 1.2上の`footnote-1`はMI3-07で公開済み。
- [ADR-0031: Advanced pagination contract and profile split](ADR-0031-advanced-pagination-profiles.md) — M3 advanced-pagination targetとしてAccepted。contract 1.3と`header-footer-1`、`columns-1`、`float-1`を予約し、MI3-12までは非公開。
