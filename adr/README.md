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
- [ADR-0031: Advanced pagination contract and profile split](ADR-0031-advanced-pagination-profiles.md) — M3 advanced-paginationとしてAccepted。MI3-12がcurrent contract 1.3とpublic `header-footer-1`、`columns-1`、`float-1`を一括公開。
- [ADR-0032: M4 semantic container and declared-media contract](ADR-0032-semantic-container-and-declared-media.md) — M4 targetとしてAccepted。non-current contract 1.4、`production-book-1`、closed semantic container、required declared mediaを予約し、MI4-13までは非公開。
- [ADR-0033: Math, safe-vector, and alternative binding](ADR-0033-math-safe-vector-and-alternative-binding.md) — M4 targetとしてAccepted。`typaxis-math` source/producer speech/visual receiptと`svg-safe-1` subsetを固定し、MI4-04/05はMI4-13まで非公開stagingで実装する。
- [ADR-0034: Document metadata, language, and outline binding](ADR-0034-document-metadata-language-and-outline.md) — M4 targetとしてAccepted。closed metadata、stable BCP 47 inheritance、source-bound outline/named-destination、Info/XMP/catalog mappingを固定し、MI4-07はMI4-13まで非公開stagingで実装する。
- [ADR-0035: Tagged PDF structure and accessibility validation](ADR-0035-tagged-pdf-structure-and-validation.md) — M4 targetとしてAccepted。PDF/UA-1 role tree、source reading order、selected-paint/MCID/ParentTree closure、artifact policy、`book-xmp/2`、veraPDF 1.30.2とMatterhorn 1.1 evidenceを固定し、MI4-09はMI4-13まで非公開stagingで実装する。
- [ADR-0036: JPEG and OpenType/CFF resource profiles](ADR-0036-jpeg-and-opentype-cff-resource-profiles.md) — M4 targetとしてAccepted。独立した`jpeg-baseline`/`sfnt-cff1` component、bounded decode、deterministic metadata strip/CID subset、embedding permission、PDF plan、exact dependency/limit policyを固定し、MI4-11/12はMI4-13まで非公開stagingで実装する。
- [ADR-0037: Producer-composed math-vector placement](ADR-0037-producer-composed-math-vector.md) — M4 targetとしてAccepted。組版済み`svg-safe-2`、4つの明示kind、producer metric/baseline、atomic inline/block layout、content-key Form dedupe、SafeVector/resource-set・book-navigation・tagged-PDFのversioned `/2`経路を固定する。MI4-V03〜V19は非公開staging/evidenceであり、MI4-13だけが公開する。

- [ADR-0038: VMB book production compatibility correction](ADR-0038-vmb-book-production-compatibility.md) — Safe-SVG 2のタグ末尾空白に限る仕様訂正、書籍用既定予算、元位置を保持する診断。実装・全巻検証は別途記録する。

- [ADR-0039: Book-2 semantic vocabulary and version-bound carrier](ADR-0039-book-2-semantic-vocabulary.md) — 非公開1.5の囲み・解答・引用をclosed kindとして保持し、1.4の語彙と公開入口を維持する。共通組版と全巻受入は別途必要。

- [ADR-0040: Book-2 authored description lists](ADR-0040-book-2-description-lists.md) — 非公開1.5で用語と説明の独立した所有者を保持する。carrier の受理と組版・PDF 対応を区別し、旧1.4のリスト形式を変更しない。

- [ADR-0041: Book-2 source number bindings](ADR-0041-book-2-source-number-bindings.md) — 非公開1.5の番号参照を実際に表示する番号の文字範囲と所有者へ結び付ける。番号の推測や旧契約の拡張は行わない。

- [ADR-0042: Book-2 source table captions](ADR-0042-book-2-table-captions.md) — 非公開1.5の表captionを元の独立ブロックとして保持する。行・セルと区別し、入力形式の受理と共通組版・PDFへの接続を段階ごとの証拠で確認する。

- [ADR-0043: Book-2 table-cell style inheritance](ADR-0043-book-2-table-cell-inheritance.md) — 元セルへclassを保持し、文字揃えと文字スタイルを子blockへ継承する。旧契約と表captionの継承範囲を維持する。

- [ADR-0044: Book-2 table-caption forced breaks](ADR-0044-book-2-table-caption-forced-breaks.md) — captionの元改ページを消費済み項目番号で追跡し、空白ページ・脚注継続・元sourceの一回消費を保持する。
- [ADR-0045: Book-2 parallel cell breaks](ADR-0045-book-2-parallel-cell-breaks.md) — セルごとの元content位置で行を継続し、隣の未配置行と同時改ページの元ownerを保持する。

- [ADR-0046: Book-2 spanning cell breaks](ADR-0046-book-2-spanning-cell-breaks.md) — rowspanの元行高とセル継続を保持し、後の行の改ページが先行描画を越える候補を再計算する。

- [ADR-0047: Book-2 original header breaks](ADR-0047-book-2-header-source-breaks.md) — 元headerの改ページを一度だけ消費し、後続の反復headerを意味上の原文と区別して描画する。

- [ADR-0048: Book-2 nested body fragments](ADR-0048-book-2-nested-body-fragments.md) — 子表の元カーソル・fragment・反復属性を親セルへ受け渡し、並列セルの容量と共有予算で継続する。親の残りの形式は段階的に接続する。

- [ADR-0049: Book-2 nested parent keeps](ADR-0049-book-2-nested-parent-keeps.md) — 親セルの保持連鎖と子表終端を巻き戻し、元ownerと共有探索予算を保持する。
- [ADR-0050: Book-2 nested caption fragments](ADR-0050-book-2-nested-caption-fragments.md) — 親captionの子表継続を本文セルから分離し、末尾keepを本文の元source開始まで保持する。
- [ADR-0051: Book-2 nested header regions](ADR-0051-book-2-nested-header-regions.md) — 元headerを一回消費し、セルのない子captionも反復属性付きで描画する。
- [ADR-0052: Book-2 nested spanning rows](ADR-0052-book-2-nested-spanning-rows.md) — 親rowspanの残り行高と子表カーソルを別に保持し、後続行の早い改ページで親候補を再選択する。
- [ADR-0053: Book-2 definition table demands](ADR-0053-book-2-definition-table-demands.md) — 元の表カーソルと脚注要求を同じ状態・予算で継続し、選択した元セルだけから依存要求を作る。
- [ADR-0054: Book-2 definition candidates](ADR-0054-book-2-definition-candidates.md) — 脚注の通常本文と表の候補を共有予算で全列挙し、同点では空表を含む元sourceの進行を優先する。
- [ADR-0055: Book-2 shared definition queue](ADR-0055-book-2-shared-definition-queue.md) — 表の継続と元番号の消費状態を共通の脚注要求へ保持し、別の脚注の処理後も同じsourceから再開する。

- [ADR-0056: Mixed footnote content in the common region selection](ADR-0056-book-2-mixed-footnote-regions.md)

- [ADR-0057: Reserve mixed footnote fragments with dependency backtracking](ADR-0057-book-2-mixed-footnote-reservations.md)

- [ADR-0058: Place definition tables on stable physical pages](ADR-0058-book-2-definition-table-pages.md)

- [ADR-0059: Retain natural cell continuations when common cuts cannot fit](ADR-0059-book-2-natural-cell-continuations.md)

- [ADR-0060: Use each selected physical page master in PDF geometry](ADR-0060-book-2-selected-page-masters.md)

- [ADR-0061: Retain actual page frames separately from measurement envelopes](ADR-0061-book-2-variable-page-frames.md)

- [ADR-0062: Select physical masters from authored body page scopes](ADR-0062-book-2-named-page-scopes.md)

- [ADR-0063: Retain and select explicit page names on source breaks](ADR-0063-book-2-explicit-break-page-names.md)

- [ADR-0064: Translate measured content to selected horizontal page origins](ADR-0064-book-2-horizontal-page-origins.md)

- [ADR-0065: Bind variable inline widths to original source starts](ADR-0065-book-2-source-start-inline-widths.md)

- [ADR-0066: Rebind source widths through actual shaping feedback](ADR-0066-book-2-source-width-reshape.md)

- [ADR-0067: Return physical paragraph widths to original source starts](ADR-0067-book-2-paragraph-page-width-feedback.md)

- [ADR-0068: Converge paragraph layout against selected physical page widths](ADR-0068-book-2-variable-page-widths.md)

- [ADR-0069: Rebind block parent frames to selected physical widths](ADR-0069-book-2-block-page-widths.md)

- [ADR-0070: Reproject source table hierarchies at candidate parent widths](ADR-0070-book-2-root-table-width-frames.md)

- [ADR-0071: Feed selected physical widths into root-table remeasurement](ADR-0071-book-2-table-page-widths.md)

- [ADR-0072: Inherit remeasured table parents for block geometry](ADR-0072-book-2-table-block-widths.md)

- [ADR-0073: Observe continued table widths by original source positions](ADR-0073-book-2-table-width-occurrences.md)

- [ADR-0074: Remeasure table occurrence frames from original hierarchy](ADR-0074-book-2-table-occurrence-frames.md)

- [ADR-0075: Bind source-unit origins through shaping and page placement](ADR-0075-book-2-source-unit-starts.md)

- [ADR-0076: Converge heterogeneous table paragraphs using source profiles](ADR-0076-book-2-table-source-profiles.md)

- [ADR-0077: Rebind table-local block widths and origins from physical occurrences](ADR-0077-book-2-table-block-source-profiles.md)

- [ADR-0078: Retain converged line contexts for independent physical variants](ADR-0078-book-2-line-variant-seeds.md)

- [ADR-0079: Rebuild source-compatible line variants in one bounded owner](ADR-0079-book-2-line-variant-sets.md)

- [ADR-0080: Bind repeated-header geometry to its actual line variant](ADR-0080-book-2-table-header-variants.md)

- [ADR-0081: Select table continuations with actual variant header capacity](ADR-0081-book-2-table-header-selection.md)

- [ADR-0082: Choose repeated headers from physical body and footnote frames](ADR-0082-book-2-table-header-catalog.md)

- [ADR-0083: Retain actual header owners through mixed-page placement](ADR-0083-book-2-table-header-placement.md)

- [ADR-0084: Close repeated header paint against original logical units](ADR-0084-book-2-table-header-source-closure.md)

- [ADR-0085: Keep repeated-header frame observations separate from semantic widths](ADR-0085-book-2-table-header-width-feedback.md)

- [ADR-0086: Resolve header math terminals through each actual measurement](ADR-0086-book-2-table-header-math-terminals.md)

- [ADR-0087: Draw headers and enumerate fonts through their actual fragment flows](ADR-0087-book-2-table-header-display.md)
- [ADR-0088: Close resource uses from actual repeated-header displays](ADR-0088-book-2-table-header-resources.md)
- [ADR-0089: Assemble PDFs from verified actual header variants](ADR-0089-book-2-table-header-pdf.md)

- [ADR-0090: Discover and converge repeated table header widths in the private driver](ADR-0090-book-2-automatic-table-header-catalog.md)

- [ADR-0091: Keep shared carrier and error enums stable under Cargo feature unification](ADR-0091-book-2-feature-unification.md)

- [ADR-0092: Restrict table remeasurement to the sources actually used at that width](ADR-0092-book-2-table-source-reachability.md)

- [ADR-0093: Retain independent header measurements through nested table continuations](ADR-0093-book-2-independent-nested-headers.md)

- [ADR-0094: Share compatible header replays and retain prepaid catalog ownership](ADR-0094-book-2-shared-header-replay.md)

- [ADR-0095: Prepare and shape source-bound header/footer text independently](ADR-0095-book-2-page-region-text-flow.md)
