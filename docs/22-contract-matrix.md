# Cross-layer contract matrix

The evidence matrix below does not by itself report delivery completion. Machine input uses four independent status axes:

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| reference TSF build | Yes, current 1.4 | Yes, bounded reference subset | Yes | No |
| DocumentPackage portable validation/export | Yes, current 1.4 plus frozen 1.0/1.1/1.2/1.3 input | Yes: independent Schema registries/validator and resource-attested `dump-ast` encoder | Yes, package commands | Yes |
| sealed package/source ingestion | Yes, ADR-0027 | Yes | Yes, macOS/Linux fixture gate | Yes, M1 host gate |
| `typaxis.machine-pdf/paragraph-1` | Yes, immutable capability contract | Yes | Yes, macOS/Linux combined PDF/sidecars | Yes |
| `typaxis.machine-pdf/basic-document-1` | Yes, ADR-0028 immutable M2 profile | Yes: canonical multi-flow, typed block-style, list, forced-page-break, PNG figure, and link annotation/named-destination pipeline | Yes, combined PDF/sidecars | Yes |
| `typaxis.machine-pdf/table-1` | Yes, ADR-0029 immutable M3 profile on contract 1.2 | Yes: resolved grid/cell flows, row fragmentation/header repetition, and Display/PDF closure | Yes, combined PDF/sidecars | Yes, MI3-04 gate |
| `typaxis.machine-pdf/footnote-1` | Yes, ADR-0030 immutable M3 profile on contract 1.2 | Yes: first-reference reflow, dedicated carry, Display/PDF, and trace/manifest closure | Yes, combined PDF/sidecars | Yes, MI3-07 gate |
| `typaxis.machine-pdf/header-footer-1` | Yes, ADR-0031 on contract 1.3 | Yes: master/subflow/box/paint closure | Yes, combined PDF/sidecars | Yes, MI3-12 gate |
| `typaxis.machine-pdf/columns-1` | Yes, ADR-0031 on contract 1.3 | Yes: exact partition/sequential fill/bounded balance/paint closure | Yes, combined PDF/sidecars | Yes, MI3-12 gate |
| `typaxis.machine-pdf/float-1` | Yes, ADR-0031 on contract 1.3 | Yes: FIFO queue/placement/carry/paint closure | Yes, combined PDF/sidecars | Yes, MI3-12 gate |
| generated contract 1.3 artifacts | Yes | Yes: config/trace/diagnostics/manifest/package/capabilities/evidence | Yes | Yes |
| contract 1.2 compatibility | Yes, ADR-0028 migration table | Yes: frozen complete independent `schemas/1.2/` registry | Compatibility input | Frozen |
| contract 1.3 publication | Yes, ADR-0031 | Yes: frozen complete independent registry and seven preserved profiles | Compatibility/old-profile artifacts | Frozen after MI4-13 |
| contract 1.4 / `production-book-1` | Yes through ADR-0037: base/media, native math/safe-vector, producer-composed math vector, metadata/language/outline, tagged PDF/PDF/UA-1 validation, and separate baseline-JPEG/CFF1 resource components | Yes, complete receipt/artifact closure | Yes, combined public fixture | Yes, MI4-13 gate |

`Contract-defined` does not imply that a Rust owner exists, the current `build` accepts DocumentPackage JSON, public CLI E2E passes, or a release supports the feature.

| Contract | Rust | JSON | Docs | Validator |
|---|---|---|---|---|
| product/CLI identity | `typaxis_core::PRODUCT_NAME` / Cargo `[[bin]]` | manifest `engine.name` | docs/19 | exact name/bin/Schema checks |
| wire ID | current `typaxis_core::CONTRACT` is 1.4; typed public DocumentPackage input IDs are 1.0 through 1.4; ADR-0032〜0037 fix the complete production semantics | current roots use 1.4; complete 1.0, 1.1, 1.2, and 1.3 registries are frozen separately; `schemas/1.4/` is the current twenty-nine-schema registry | contract-version, ADR-0027 through ADR-0037 | MI4-V19 closed feature readiness and MI4-13 switched every alias atomically |
| source/text/local map range | `SourceSpan` / `TextSpan` / `Utf8ByteRange` | common + document package | docs/03 | bounds/boundary/coverage |
| generated/Display text ownership | `GeneratedBufferKey` / `GeneratedTextStore` / `DisplayTextMap` / `DisplayDocument.text_buffers` | display text buffers/spans | docs/05,09,11 | canonical key allocation + disjoint internal IDs + selected-bound stable dense remap + artifact-owned text table |
| validated parser output | sealed `Parser` / `ValidatedParsedPackage` / `ParseOutcome` / `AdvisoryDiagnostic` | N/A (in-process) | docs/01,03 | source-driven owner + no feature promotion + compile-fail boundary + error-or-fatal/value exclusion |
| host/path admission | `HostAdmissionContext` / `BuildExecutionContext` / `ConfigResourceRoot` / `PortablePath` | portable path + config roots | docs/01,18,19 | ProjectRoot variant + containment + 0/1/>1 candidate result + no serialized HostPath |
| URI admission | `SafeUri` | typed URI fields | docs/03,15,18 | scheme/control/whitespace/length |
| length and transform | `Length` / `AffineTransform` | common defs | docs/24 | numeric/type checks |
| parser package | `ParsedPackage` | document root | docs/03,04 | Rust token + Schema |
| machine package ingestion | stable-byte admission, strict decoder, sealed source validation, and session-bound receipts are implemented; `WireDocumentPackage` remains untrusted; 1.4 adds semantic container, required declared media including `jpeg-baseline`/`sfnt-cff1`/`svg-safe-2`, native and producer-vector math/source/alternative, metadata/language, and outline; ADR-0035/0037 version the PDF/structure projection | public 1.0 through 1.4 input and current 1.4 output; old-profile artifacts use frozen 1.3 dispatch | ADR-0027, ADR-0028, ADR-0031 through ADR-0037, docs/02,19,25,26, contracts/phase-ownership | independent Schema/semantic validation, public E2E, legacy rejection, and resource-attested source export |
| machine PDF capability | exact eight-profile descriptors with matching preflight receipts | current 1.4 capability Schema and canonical fixture include `production-book-1`, required PDF/UA-1, exact `typaxis.production-book-resource-set/2`, and the closed vector kind/metric/profile/feature projection | contracts/machine-pdf-capabilities, ADR-0027 through ADR-0037, docs/26 | bidirectional descriptor/combined-fixture closure, `m3-all.json`, and `m4-production.json`; default and old-profile meanings remain frozen |
| basic-document profile | MI2-02 multi-flow owners, MI2-03 typed block-style receipts/consumers, MI2-04 syntax-owned marker/list receipts, MI2-05 forced-boundary receipts, MI2-06 admitted-PNG/figure-placement/DrawImage/XObject receipts, and MI2-07 package-bound link/cluster/rectangle/annotation receipts | frozen 1.2 DocumentPackage semantics plus their exact neutral 1.3 encoding and versioned selected facts | ADR-0028, contracts/machine-pdf-capabilities, docs/25,26 | combined all-advertised fixture, typed closure, deterministic PDF goldens, exact limits, receipt swaps, and tamper negatives |
| table profile | MI3-02/MI3-03 resolved columns/grid, canonical cell FlowIds, row bands/fragments, rowspan continuation, and header repetition; MI3-04 exact Display-command and frozen-PDF graph closure | unchanged 1.2 `table` wire or neutral 1.3 encoding plus conditional table trace/manifest facts; no table-specific style field | ADR-0029, contracts/machine-pdf-capabilities, docs/10,25,26 | public `m3-table.json` table-only/combined coverage, exact limits, receipt and command tamper negatives, zero-decoration PDF/raster, reproducibility, and old-profile rejection gate |
| footnote profile | canonical FootnoteFlowIds, first-reference discovery, exact reservation/evaluation/convergence, dedicated carry-only pages, selected layout, definition-anchor/link paint, and Display/PDF closure | unchanged 1.2 definition/reference/page-master-footnote wire or neutral 1.3 encoding plus conditional `machine-footnote-manifest` facts; no footnote-specific style field | ADR-0030, contracts/machine-pdf-capabilities, docs/04,08,09,10,25,26 | public `m3-footnote.json`: zero and combined M2, catalog-vs-first-reference order, repeat/split/multi-page carry, receipt/paint tamper, PDF/raster/text order, reproducibility, and old-profile rejection |
| advanced-pagination profiles | MI3-09 implements checked master/trim/page boxes, canonical first/left/right selection, and independent header/footer flows; MI3-10 implements exact residual columns, canonical parent/source FlowIds, monotonic sequential fill, bounded final-page balance, and selected/Display/PDF/manifest closure; MI3-11 implements canonical FIFO float anchors, typed here/top/bottom/next-page decisions, nonwrapping exclusion bands, bounded page carry, and placement/object closure; MI3-12 binds admitted content/resources and exposes normal public dispatch | frozen `schemas/1.3/` covers the advanced DocumentPackage additions and exact conditional `machine-advanced-pagination-manifest` | ADR-0031, contracts/machine-pdf-capabilities, contracts/phase-ownership, docs/10,25,26 | three all-advertised combined fixtures, exact/max+1 and progress gates, public G6003/G6004, PDF boxes/raster/text, reproducibility, old-profile freeze, and aggregate `m3-all.json` |
| semantic container | closed `result`, `proof`, `exercise`, NodeId/SourceSpan/child ownership, `semantic_container` style scope, one canonical container FlowId, strict page-fragment progress, and one outline/tag structure boundary | current 1.4 includes required nullable `anchor_id`, optional semantic `language`, and the complete selected/PDF projection | ADR-0032, ADR-0034, contracts/machine-pdf-capabilities, contracts/phase-ownership, docs/25 | nested/split/empty/unknown/wrong-owner/round-trip/tamper fixtures plus MI4-13 public closure |
| declared media | provenance-bound `LegacyUnspecified` versus typed `Declared`; machine profile owns allowed set; resource admission alone issues and exact-matches attestation; source exporter consumes that same receipt | 1.4 requires images `png`, `svg-safe-1`, `svg-safe-2`, or `jpeg-baseline` and fonts `sfnt-truetype-glyf`, `ttc-truetype-glyf`, or `sfnt-cff1`; manifest declaration and attestation remain distinct | ADR-0032, ADR-0033, ADR-0036, ADR-0037, contracts/contract-version, contracts/phase-ownership, docs/25 | public base/SafeVector/JPEG/CFF closure, stable attestation, old-profile freeze, and legacy pre-resource rejection |
| math/safe-vector | exact `typaxis-math` version `1` source/span plus producer speech, opaque MathReceiptKey, admitted MATH font, atomic inline or dedicated display MathFlowId, selected vector paint/ActualText/Formula closure; stable-byte-only bounded Safe-SVG decoder and canonical IR/usage/Form chain | current 1.4 includes declared/attested `svg-safe-1`, `inline_math`, `display_math`, SafeVector manifest, selected placement, PDF `/ActualText`, and Formula structure | ADR-0033, ADR-0035, contracts/machine-pdf-capabilities, contracts/phase-ownership, docs/25 | exact/max+1, tamper, PDF, extraction, old-profile isolation, and combined M4 publication gates |
| producer-composed vector | explicit `inline_vector`, `math_vector`, `vector_figure`, `math_vector_block`; fixed-point advance/ascent/descent/origin/baseline/viewport and spacing; opaque TeX/Alt/ActualText; Safe-SVG 2 currentColor/alpha; atomic line/block/number layout; verified-content-key Form/ExtGState dedupe; navigation/tagged-PDF `/2` closure | MI4-V01/V02 fix the corpus/decision and MI4-V03〜V19 implement the product path plus external/readiness evidence | ADR-0037, docs/27, contracts/machine-pdf-capabilities, contracts/phase-ownership | MI4-V19 closes negative/tamper/determinism and external evidence; MI4-13 publishes the complete descriptor/resource-set `/2` |
| JPEG/CFF resources | distinct immutable PNG, SafeVector, baseline-JPEG, TrueType-glyf, and standalone-CFF1 component IDs compose one ordered production resource set; typed IDs bind every receipt | MI4-11 and MI4-12 implement separate admission, transform/subset, PDF, manifest, exact-limit, renderer/extractor, dependency-audit, and old-profile gates | ADR-0036, contracts/contract-version, contracts/machine-pdf-capabilities, contracts/phase-ownership, docs/25 | MI4-13 combines and advertises both only in resource-set `/2` |
| metadata/language/outline | explicit all-null-or-producer metadata, canonical UTC-second dates, registry-independent BCP 47 inheritance, source-preorder outline, exact AnchorId/destination binding, and fixed Info/XMP/catalog/outline bytes | current 1.4 production Schema/Rust/PDF/independent-validator path; no old profile gains the fields | ADR-0034, contracts/contract-version, contracts/machine-pdf-capabilities, contracts/phase-ownership, docs/25 | exact/max+1, JSON Pointer diagnostics, no clock/host/path inference, receipt tamper, deterministic/path-alias fixture, and public closure |
| tagged PDF/accessibility | source-bound exhaustive roles/generated wrappers, selected structure-or-artifact receipts, dense page-local MCIDs, complete PDF structure graph, alternatives/language/relations, and `book-xmp/2` | current production structure/Display/PDF/manifest/in-tree-validator path; MI4-V19 binds it to veraPDF and reviewed Matterhorn ledger | ADR-0035, contracts/contract-version, contracts/machine-pdf-capabilities, contracts/phase-ownership, docs/25 | veraPDF 1.30.2 `ua1` and complete Matterhorn 1.02 `/2` ledger bound to one PDF hash; fixture-scoped, not a universal accessibility claim |
| canonical lists | current document list type plus package-bound marker-usage, item-flow layout, selected fragment, Display/PDF, and manifest receipts | ordered/start relation plus versioned 1.2 selected list facts | docs/04, ADR-0028, docs/25 | ordered positive start + checked item index, unordered null/U+2022, marker buffer/aggregate max+1, widest-column LTR/RTL placement, nested child-frame indent, marker orphan and missing/extra/wrong-item closure |
| forced page breaks | current typed `PageBreak` plus package/epoch/FlowId-bound layout boundary and exact consume receipt | versioned 1.2 selected forced-break cursor/page facts | ADR-0028, docs/09, docs/25 | start/middle/consecutive/trailing blank policy, `N + 1` pages, exact/max+1 page limit, stale-cursor and break-paint closure; `paragraph-1` remains closed |
| non-floating PNG figures | decoder-only `AdmittedImageMediaKind::Png`, package-bound Figure/caption usage, `ValidatedFigureLayout`, selected placement, one-DrawImage Display, finalized image/soft-mask plans, and graph/serializer-bound XObject facts | frozen 1.2 Figure or required 1.3 `placement=block` plus versioned `machine-figure-manifest`; current 1.4 requires `media_type=png` without treating it as attestation | ADR-0028, ADR-0032, docs/07,13,25 | opaque-suffix stable-read admission, full bounded decode, pixel/aspect dimensions, caption split/keep/terminal oversize, hash/media mismatch, limits, XObject closure, deterministic builds, and M4 declared-media publication |
| link annotations and named destinations | syntax-owned package anchor/`SafeUri` target receipt, selected logical shaping-cluster ranges, canonical page/line visual rectangle unions, Display annotations, and graph/serializer-bound PDF observations | unchanged 1.2 link wire carried into 1.3 plus versioned `machine-link-manifest` | ADR-0028, docs/07,13,25 | wrapped internal/external links, scheme-only normalization, package-bound targets, nonempty/painted preflight, exact rectangle/object limits, missing/extra/wrong-page/wrong-target/rectangle closure, deterministic PDF golden; `paragraph-1` remains closed |
| block selectors/style cascade | current selector/cascade/`ResolvedTextStyle` plus the frozen 1.2 eight-property registry and computed receipt | block classes plus unchanged tagged declarations in frozen old-profile 1.3 and current 1.4 branches | docs/04, ADR-0025, ADR-0028 | grammar/class order, tagged-value min/max/max+1, applicability, initial/inherit/extends/override, registry/owner/package binding, typed-consumer coverage |
| page selection | `PageName` / `PageSelectionContext` / `PageContext` | page property + master rules | docs/04 | typed page value + derived flags + master winner |
| admitted resources | `AdmittedRootSet` / `VerifiedResourceSource` / `AdmittedResourceLedger(Token)` | build records | docs/04,13,18 | sealed root/session + same-handle extent/hash/metadata/full declaration closure |
| layout/shaping trust | `PackageShapeTextReceipt` / `typaxis-layout-contract::LayoutEpoch` / `ShapeFontSelectionReceipt` / private-owner `ShapeRequest` | display run | docs/02,05 | typed text site/style owner + exact parsed/generated bytes + package/reference epoch + exact-ledger/instance/computed-face bind + owner-derived adjacency/bidi/script + canonical Profile language/features; cache includes shaper/data-table versions; raw request/text/instance injection absent |
| paragraph items | `ParagraphItem` / `BreakKind` | internal IR | docs/06,07 | provenance/run slice/branch content |
| reflow | `Fragmenter` / `Continuation` | trace fragments | docs/08 | structured strict progress |
| convergence | state-indexed passes, `InitialPaginationState`, `ReferenceTransitionReceipt`, `PaginationFingerprintRecord`, score | layout trace | docs/09 | package/limits-derived initial overlay + previous fingerprint + same-session sealed working-overlay transition + per-state paint overlay + canonical chain/stable/cycle/score/selection; session capability stays in-process |
| Display ops | `DISPLAY_COMMAND_OPS` | exact enum | docs/11 | exact set comparison |
| text paint/destinations | `Paint` / `NamedDestination` | display root/run | docs/08,09,11,15 | exact selected-anchor closure, package ID/owner binding, page/frame point derivation, resolution, and bounds |
| path/stroke | `Path` / `DashPattern` | command schema | docs/11 | arity/state/dash |
| cluster extraction | `DisplayCluster` | cluster objects | docs/12 | span/overlap/order |
| subset plan | `FontSubsetPlan` | finalized internal | docs/12 | Rust plan tests |
| resource finalization | `ResourceCollector` / `FrozenPdfResourcePlans` | build records | docs/13 | selected epoch/ledger bind + usage union + admitted identity + typed indirect-object blueprint |
| PDF stream | `PdfStreamObject` | N/A | docs/14 | reserved-key source invariant |
| frozen PDF graph | untrusted builder / `FrozenPdfGraph` | N/A | docs/14,15 | full object preflight + font-role/page/annotation allocation + root/reference/page/destination/annotation/depth closure |
| limits | `ResourceLimits` / `MachineInputLimitBounds` plus `M4ResourceLimits` | current config and capability Schemas | docs/03,07,09,10,18,19,25 | exact field set, inclusive max semantics, iterative prechecks, existing limit reuse, M4 vector/math/JPEG/CFF one-time charges, and stable owning diagnostic codes |
| effective config/build manifest | `EffectiveConfig` / `BuildInputProfile` / `PackageInputRecord` / publication contexts | current 1.4 config/build Schema contains complete production media/math/navigation/tagged/vector pairs; old-profile artifact branches retain frozen 1.3 shapes | docs/16,19,25, ADR-0031 through ADR-0037 | raw 1.0 through 1.4 config normalization, terminal publication, profile-based artifact dispatch, and no M4 fields in old-profile branches |
| data/shaper/engine identity | `ResolvedDataTables` / `ShaperIdentity` / `EngineIdentity` | config/manifest identity facts | docs/05,16 | known table/shaper registry selection + build-issued engine facts; ShaperIdentity alone is not an actual-use capability |
| diagnostics | `DiagnosticCode` / `DiagnosticLocation` / `AdvisoryDiagnostic` | diagnostic pattern/severity + tagged nullable location union | docs/17,25 | exact category/severity, package JSON/source locations, located notes, canonical encoding, and outcome rules |
| machine host evidence | clean-built public binary + verifier-owned canonical evidence writer | machine-profile-evidence Schema | docs/25,26, sample README | exact check/tool/artifact sets, revision/source/fixture binding, Linux/macOS missing/failed/stale/cross-host mismatch rejection |
| archive | release builder | MANIFEST | docs/16 | metadata/order/safety/rebuild |

Contract変更時はRust、Schema、positive/negative fixture、docs、validatorを同じchange setで更新する。

## M2 contract/profile migration matrix

ADR-0028 fixes this publication behavior; implementation may not infer a newer profile or silently upgrade a wire package.

| Raw DocumentPackage contract | Profile | Before MI2-08 | After MI2-08 |
|---|---|---|---|
| 1.0 / 1.1 | omitted or `paragraph-1` | accepted under frozen paragraph semantics | accepted as compatibility input; generated artifacts are 1.2 |
| 1.2 paragraph-only semantic subset | omitted or `paragraph-1` | `P1103` because 1.2 is non-current | accepted without broadening the paragraph descriptor |
| 1.0 / 1.1 | `basic-document-1` | unknown profile usage exit 2 | `P1103` at `/contract`; typed 1.2 properties are not synthesized |
| 1.2 | `basic-document-1` | unknown profile/contract rejected by public CLI | accepted only for ADR-0028's complete closed domain |
| unknown | any | `P1103` | `P1103`; no newest-contract fallback |

The default remains `typaxis.machine-pdf/paragraph-1`. MI2-08 completed the “After” column atomically: 1.1 is frozen, every current artifact/decoder/`dump-ast` alias uses 1.2, dedicated staging runner entry points are gone, and the public descriptor plus combined fixture are registered in `m2-basic.json`.

## M3 table profile adoption matrix

ADR-0029 fixed table semantics without migrating the current wire. The middle
column preserves the pre-publication state; MI3-04 closed the final column.

| Raw DocumentPackage contract | Profile | After MI3-01 / before MI3-04 | After the MI3-04 gate |
|---|---|---|---|
| 1.0 / 1.1 | `table-1` | unknown profile usage exit 2 | `P1103` at `/contract`; no style/profile upgrade |
| 1.2 | `paragraph-1` or `basic-document-1` with table | existing `L5100` rejection | unchanged `L5100` rejection |
| 1.2 | `table-1` | contract-defined only; public CLI rejects the profile while private slices remain non-public | accepted only for ADR-0029's closed direct-body table domain |
| 1.2 with a table border/padding/alignment/background/split field | `table-1` | `P1102` as unknown current wire/style | unchanged; requires a new contract and profile |
| unknown | any | `P1103` or unknown-profile usage error | same; no newest-contract/profile fallback |

MI3-04 moved only the `table-1` status axes after descriptor/combined-fixture
bidirectional coverage, grid/rowspan/header/Display/PDF receipt closure,
inclusive `max_ast_nodes`/`max_fragments` max+1 checks, zero-decoration raster
evidence, reproducibility, and older-profile rejection goldens passed. It did
not change the current contract ID, DocumentPackage Schema bytes, or default
profile.

## M3 footnote profile adoption matrix

ADR-0030 fixes footnote semantics on the existing 1.2 wire. MI3-05 changed
only the contract-defined axis; MI3-07 closed the publication transition.

| Raw DocumentPackage contract | Profile | After MI3-05 / before MI3-07 | After the MI3-07 gate |
|---|---|---|---|
| 1.0 / 1.1 | `footnote-1` | unknown profile usage exit 2 | `P1103` at `/contract`; no profile/wire upgrade |
| 1.2 | `paragraph-1`, `basic-document-1`, or `table-1` with a footnote definition/reference | existing `L5100` rejection | unchanged `L5100` rejection |
| 1.2 | `footnote-1` | contract-defined only; public CLI rejects the profile while MI3-06 uses crate-private staging | accepted only for ADR-0030's basic-document-plus-footnote domain |
| 1.2 table plus footnote | `table-1` or `footnote-1` | existing/target closed-domain rejection | unchanged; neither standalone profile composes the other |
| 1.2 with an authored marker/separator/split/continuation/note-style field | `footnote-1` | `P1102` as unknown current wire/style | unchanged; requires a new contract and profile |
| unknown | any | `P1103` or unknown-profile usage error | same; no newest-contract/profile fallback |

MI3-07 moved the final status axes after one descriptor drove public
preflight/capabilities and bidirectional `m3-footnote.json` coverage; the
first-reference/reservation/convergence/carry/paint receipt chain closed
through trace, manifest, and PDF; focused limit and receipt-tamper tests passed;
and old-profile rejection, external PDF/raster/text-order, reproducibility, and
documented-host gates passed. Publication left contract 1.2 DocumentPackage
Schema bytes and the `paragraph-1` default unchanged.

## M3 advanced-pagination contract/profile migration matrix

ADR-0031 fixed the migration mapping. MI3-12 moved the “after” column into
public code as one atomic publication; the “before” column remains historical.

| Raw DocumentPackage contract | Profile | After MI3-08 / before MI3-12 | After the MI3-12 gate |
|---|---|---|---|
| 1.0 / 1.1 / 1.2 | omitted or `paragraph-1` | current frozen paragraph behavior; output is 1.2 | accepted with the same paragraph semantics; canonical output is 1.3 |
| neutral 1.3 paragraph subset | omitted or `paragraph-1` | public `P1103` because 1.3 is non-current | accepted only with full-media trim, null auxiliary content/columns, horizontal/LTR, and no Figure |
| non-neutral 1.3 advanced wire | omitted or `paragraph-1` | public `P1103` | `L5100`/`L5101`; default selection never upgrades a profile |
| 1.2 | `basic-document-1`, `table-1`, or `footnote-1` | accepted by the matching current profile | unchanged accepted semantic set; canonical output was then-current 1.3 |
| neutral 1.3 | `basic-document-1`, `table-1`, or `footnote-1` | public `P1103` because 1.3 is non-current | accepted as compatibility input with full-media trim, null region/columns, block Figures, and unchanged auxiliary-frame rules |
| non-neutral 1.3 | `basic-document-1`, `table-1`, or `footnote-1` | public `P1103` | `L5100`/`L5101`; no advanced feature is ignored, synthesized, or downgraded |
| 1.0 / 1.1 | `basic-document-1`, `table-1`, or `footnote-1` | existing old-contract rejection | `P1103` at `/contract`; typed 1.2 semantics are not synthesized |
| 1.3 | matching `header-footer-1`, `columns-1`, or `float-1` | profile is unknown usage exit 2 and contract is public `P1103` | accepted only for ADR-0031's profile-specific closed domain |
| 1.0 / 1.1 / 1.2 | any advanced profile | profile is unknown usage exit 2 | `P1103` at `/contract`; old packages are not upgraded into advanced pagination |
| unknown | any | `P1103` or unknown-profile usage | same; no newest-contract/profile fallback |

At the MI3-12 publication, the default remained
`typaxis.machine-pdf/paragraph-1`, `document_package_contracts` became exactly
1.0/1.1/1.2/1.3, and the canonical
profile order is `basic-document-1`, `columns-1`, `float-1`,
`footnote-1`, `header-footer-1`, `paragraph-1`, `table-1`. Existing profile
descriptor objects and table/footnote-specific projections omit
`advanced_pagination`; built new profiles require the identical canonical
selected record in trace and manifest. Neutral-1.3 compatibility preserves
the current-encoder `dump-ast -> build-package` path without expanding any old
profile's semantic domain.

The publication transaction validated the complete independent 1.3 registry,
then switched the current contract constant, Schema aliases, serializer,
decoder, `dump-ast`, normalized config, diagnostics, capability, trace,
manifest, public dispatch/help, and fixtures in one change set. It removed all
private runners in that same gate. The frozen 1.0, 1.1, and 1.2 registries are
never populated with 1.3 definitions.

## M4 production-book migration matrix

ADR-0032 fixes the base semantic-container/declared-media mapping, ADR-0033
fixes math/safe-vector binding, and ADR-0034 fixes metadata, language, and
outline binding. ADR-0035 fixes source-bound, layout-contract-owned tagged
structure, marked-content and artifact closure, and PDF/UA-1 validation
evidence. ADR-0036 fixes distinct baseline-JPEG and standalone-CFF1 components,
their transforms/subset/PDF plans, limits, and exact dependencies. ADR-0037
adds the separately versioned producer-composed vector path, resource-set `/2`,
book-navigation `/2`, and tagged-PDF `/2`. MI4-02 through MI4-12 and MI4-V03
through MI4-V19 implemented the slices in isolation; MI4-13 moved the “after”
column into public code only after MI4-V19.

“Old profile” means any of the seven profiles public before M4. Each one's
exact accepted raw-contract set is frozen: none accepts contract 1.4, even if
the package uses only that profile's previous semantic and media subset.

| Raw DocumentPackage contract | Profile | After MI4-01 / before MI4-13 | After the MI4-13 gate |
|---|---|---|---|
| 1.0 / 1.1 / 1.2 / 1.3 | omitted or matching old profile | current acceptance/rejection and 1.3 artifact bytes | unchanged; old raw-contract/profile artifact encoders and manifest goldens remain frozen 1.3 |
| 1.4 | omitted or any old profile | public `P1103` because 1.4 is non-current | `P1103` at `/contract` before resource open; omission remains `paragraph-1`, and no old profile gains a new accepted contract |
| 1.4 | `production-book-1` | unknown profile usage and public `P1103` | accepted only for the complete M4 domain adopted before publication, with every resource declaration `Declared` |
| 1.0 / 1.1 / 1.2 / 1.3 | `production-book-1` | unknown profile usage | frozen provenance lowers absence to `LegacyUnspecified`, then profile preflight rejects before resource open; new failed manifest may record legacy/null |
| 1.4 missing/null/unknown image or font `media_type` | any | public `P1103` | `P1102` during decode; missing/unknown never becomes `LegacyUnspecified` |
| unknown | any | `P1103` or unknown-profile usage | same; no newest-contract/profile fallback |

The semantic-container record has exactly the closed
`result|proof|exercise` kind, required nullable `anchor_id`, and a nonempty
block list. It is block-only, owns one canonical FlowId, preserves one typed
wrapper and the
`/Result|/Proof|/Exercise -> /Div` structure mapping across page fragments,
and has no paragraph/class/text/raster fallback. Unknown/structural/source
errors are `P1102`; semantic empty/unsupported profile use is `L5100`;
inapplicable style is `L5101`; and
closure/progress contradiction is `I9190`.

The base declared-media mapping is exactly image `png` and fonts
`sfnt-truetype-glyf` / `ttc-truetype-glyf`. Syntax alone may issue
provenance-bound legacy absence; profile preflight owns allowed declarations;
resource admission alone issues bytes-derived attestation; and source-mode
`dump-ast` populates 1.4 only from that same stable attestation. URI suffixes,
caller JSON, PDF objects, and manifests are not format authority.

MI4-04 implemented ADR-0033's additional image value `svg-safe-1`.
Stable bytes, declaration, hash, M4 limits, and profile identity are bound to a
bounded in-tree decoder receipt and canonical IR before selected Figure,
`DrawVector`, Form plan, PDF Form XObject, and SafeVector manifest closure.
Unused admitted vectors remain manifest resources but allocate no Form plan or
PDF object. The M4 limit extension is now part of current 1.4 effective config
and remains absent from frozen old-profile artifact/config shapes.

ADR-0037 adds `svg-safe-2` only for the new producer-composed path and keeps
the existing `svg-safe-1` parser/IR/Display/Form/PDF/manifest `/1` chain
byte-frozen. `inline_vector` and `vector_figure` accept either safe-vector
media; `math_vector` and `math_vector_block` require `svg-safe-2`, exact opaque
source TeX, meaningful Alt, nullable authored ActualText, and canonical
producer metrics. `svg-safe-2` requires expected full-byte SHA-256 and
engine/version/rules provenance and adds only exact currentColor plus per-paint
fill/stroke alpha to the closed Safe-SVG 1 grammar. General SVG, `use`, CSS,
text/font/image, script/animation, external/data/file/network reference,
group opacity, mask/filter/blend, and unknown features remain terminal errors.

Inline vectors are indivisible AL/isolate items: line width uses `advance`,
producer baseline aligns to text baseline, exact same-line spacing does not
create a break, and maximum ascent/descent determines line height. Block
vectors use one uniform viewport scale, typed alignment/spacing/keep, an
independent source-owned equation-number rectangle when present, and atomic
pagination. Logical/visual empty-frame overflow is `L5100`; no split, shrink,
crop, raster, native-math, or warning fallback exists.

Verified full-byte/parser/IR content keys, not resource IDs or first use, order
and deduplicate Form/ExtGState plans. The complete resource set is
`typaxis.production-book-resource-set/2`, preserving the ADR-0036 component
order while replacing only SafeVector with `/2`; image media order is
`png, svg-safe-1, svg-safe-2, jpeg-baseline`. Computed language,
book-navigation, structure/marked-content/validator, and tagged manifest use
the adopted `/2` chains; native math and all `/1` records remain frozen.
MI4-V01 contains corpus evidence only. MI4-V03 through V18 owned isolated
product slices, MI4-V19 depended on MI4-11/12 and closed external feature
evidence, and MI4-13 advertised the complete vector capability projection.

ADR-0036 adds exact declarations `jpeg-baseline` and `sfnt-cff1`; no
generic alias exists. JPEG admits only one 8-bit Huffman baseline JFIF frame/
scan with Gray or closed YCbCr sampling. It rejects EXIF/ICC/APP metadata,
orientation, progressive/extended/lossless/arithmetic coding, ambiguous color,
and trailing streams. Bounded preflight precedes exact-pinned platform-
independent decode; the deterministic transform removes only the mandatory
JFIF APP0 and embeds the otherwise identical DCT stream with explicit
DeviceGray/DeviceRGB and ColorTransform.

The CFF component admits only standalone `OTTO`, face zero, the exact table
set, and name-keyed CFF1. It rejects bare/collection/CFF2/variation/color/
bitmap/vertical/unknown forms. Only fsType 0, 0x0004, or 0x0008 passes before
shaping. Selected glyph receipts plus `.notdef` form an ascending dense map;
bounded iterative Type 2 evaluation emits one deterministic hint/subroutine-
stripped CID-keyed `OTTO` subset for FontFile3/OpenType and CIDFontType0. Six
inclusive font-work limits use `R7130` through `R7135`. MI4-11/12 implemented
these paths separately in isolated staging; MI4-13 published their composition
without modifying frozen surfaces.

ADR-0034 requires the closed seven-field metadata record, required document
language, optional semantic-node language overrides, and explicit outline
array in the complete target shape. Metadata and UTC-second dates are exact
producer facts; canonical BCP 47 tags inherit through logical owners; outline
entries bind heading/container NodeIds to their exact package AnchorIds and
the existing selected named-destination registry. Info, fixed XMP, catalog and
marked-content language, and outline objects consume that one receipt chain.
Clock, file time, host/path, locale, first-heading, page, and coordinate
inference is forbidden. MI4-07 implemented the isolated
Schema/Rust/PDF/validator closure and MI4-13 published it without changing old
profiles.

ADR-0035 consumes those existing semantic facts without adding a Wire member.
One layout-contract-owned pre-layout structure registry exhaustively maps
Document, headings, paragraphs, semantic containers, lists/items,
tables/sections/rows/cells, Figures/captions, inline/display Formulae,
Notes/references, text emphasis, Links, anchors, and breaks to a fixed
PDF/UA-1 role or explicit no-element case. Generated wrapper keys and dense
StructureNodeIds are canonical; source parentage and `/K` order remain logical
even when columns, floats, page splits, repeated headers, or footnote regions
change physical paint order.

Selected layout binds every occurrence to one structure fragment or a closed
Pagination/Layout artifact. Display binds exact paint IDs, and the marked-
content planner alone groups final paint and allocates dense page-local MCIDs.
PDF serializes only that receipt into StructTreeRoot, RoleMap, MCR/OBJR kids,
ParentTree/IDTree, page and annotation parent keys, structure language,
alternatives, table headers, Note/reference relations, Link Contents, and
outline `/SE`. The PDF/UA projection versions XMP as `typaxis.book-xmp/2`
without changing MI4-07's version-1 bytes. Generated nodes/depth,
MCR/artifacts, strings, MCIDs, objects, output, and spool reuse existing
inclusive limits; closure failure is `I9190` and has no untagged, artifact, or
warning fallback.

MI4-09 implemented the isolated structure/Display/PDF/manifest and independent
validator slice. A release claim additionally requires exact veraPDF 1.30.2
`ua1` success with no warnings and a complete Matterhorn Protocol 1.02 `/2` assessment ledger
bound to the same PDF hash. Machine validation alone cannot establish the
human semantic checks or a broader legal/accessibility claim. MI4-13 consumed
the completed MI4-V19 evidence as its publication gate.

Only the 1.4 production-manifest branch adds tagged `media_declaration` and M4
font attestation. Built M4 records require declared/non-null exact match.
Current image records already have required PNG `attested_media_kind`; frozen
old image/font shapes remain unchanged. Legacy/null is limited to an
old-contract M4 request rejected before resource admission. Existing old
raw-contract/profile artifacts stay on the frozen 1.3 registry; raw 1.4 or any
M4 profile request uses 1.4 version dispatch.

MI4-13 validated the complete independent 1.4 registry, then atomically
switched contract decode, versioned encode, resource-attested `dump-ast`,
config, diagnostics, manifest dispatch, current Schema aliases,
capability/profile/help, fixtures, and evidence. The default remains
`paragraph-1`; the canonical profile order contains `production-book-1` once
between it and `table-1`. Raw 1.4 requires explicit production-profile
selection and normal closed-domain preflight; no old profile gains contract
1.4, and no 1.4 definition was added to frozen 1.0 through 1.3 registries.
