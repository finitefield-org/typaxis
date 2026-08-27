# Cross-layer contract matrix

The evidence matrix below does not by itself report delivery completion. Machine input uses four independent status axes:

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| reference TSF build | Yes, current 1.2 | Yes, bounded reference subset | Yes | No |
| DocumentPackage portable validation/export | Yes, current 1.2 plus frozen 1.0/1.1 input | Yes: independent Schema registries/validator and shared `dump-ast` encoder | Yes, package commands and round trip | Yes |
| sealed package/source ingestion | Yes, ADR-0027 | Yes | Yes, macOS/Linux fixture gate | Yes, M1 host gate |
| `typaxis.machine-pdf/paragraph-1` | Yes, immutable capability contract | Yes | Yes, macOS/Linux combined PDF/sidecars | Yes |
| `typaxis.machine-pdf/basic-document-1` | Yes, ADR-0028 immutable M2 profile | Yes: canonical multi-flow, typed block-style, list, forced-page-break, PNG figure, and link annotation/named-destination pipeline | Yes, combined PDF/sidecars | Yes |
| `typaxis.machine-pdf/table-1` | Yes, ADR-0029 immutable M3 profile on contract 1.2 | Yes: resolved grid/cell flows, row fragmentation/header repetition, and Display/PDF closure | Yes, combined PDF/sidecars | Yes, MI3-04 gate |
| `typaxis.machine-pdf/footnote-1` | Yes, ADR-0030 immutable M3 profile on contract 1.2 | Yes: first-reference reflow, dedicated carry, Display/PDF, and trace/manifest closure | Yes, combined PDF/sidecars | Yes, MI3-07 gate |
| `typaxis.machine-pdf/header-footer-1` | Yes, ADR-0031 target on contract 1.3 | Yes: private MI3-09 master/subflow/box/paint vertical slice | No; public ID rejected | No, MI3-12 gate |
| `typaxis.machine-pdf/columns-1` | Yes, ADR-0031 target on contract 1.3 | Yes: private MI3-10 exact partition/sequential fill/bounded balance/paint vertical slice | No; public ID rejected | No, MI3-12 gate |
| `typaxis.machine-pdf/float-1` | Yes, ADR-0031 target on contract 1.3 | No | No; public ID rejected | No, MI3-12 gate |
| generated contract 1.2 artifacts | Yes | Yes: config/trace/diagnostics/manifest/package/capabilities/evidence | Yes | Yes |
| contract 1.2 publication | Yes, ADR-0028 migration table | Yes: current aliases plus the complete independent `schemas/1.2/` registry; former 1.1 is frozen | Yes | Yes |
| contract 1.3 advanced-pagination target | Yes, ADR-0031 | Partial: private header/footer and columns DTO/Schema/owners/artifacts; float pending | No; current aliases remain 1.2 | No, MI3-12 gate |

`Contract-defined` does not imply that a Rust owner exists, the current `build` accepts DocumentPackage JSON, public CLI E2E passes, or a release supports the feature.

| Contract | Rust | JSON | Docs | Validator |
|---|---|---|---|---|
| product/CLI identity | `typaxis_core::PRODUCT_NAME` / Cargo `[[bin]]` | manifest `engine.name` | docs/19 | exact name/bin/Schema checks |
| wire ID | current `typaxis_core::CONTRACT` is 1.2; typed public DocumentPackage input IDs are 1.0/1.1/1.2; ADR-0031 reserves non-current 1.3 | current roots use 1.2; complete 1.0 and 1.1 registries are frozen separately; `schemas/1.3/` is a private three-schema staging subset | contract-version, ADR-0027, ADR-0028, ADR-0029, ADR-0030, ADR-0031 | independent frozen 1.0/1.1 and current/versioned 1.2 registries plus private MI3-09/MI3-10 validation; MI3-12 must complete/freeze independent 1.3 before atomically switching aliases |
| source/text/local map range | `SourceSpan` / `TextSpan` / `Utf8ByteRange` | common + document package | docs/03 | bounds/boundary/coverage |
| generated/Display text ownership | `GeneratedBufferKey` / `GeneratedTextStore` / `DisplayTextMap` / `DisplayDocument.text_buffers` | display text buffers/spans | docs/05,09,11 | canonical key allocation + disjoint internal IDs + selected-bound stable dense remap + artifact-owned text table |
| validated parser output | sealed `Parser` / `ValidatedParsedPackage` / `ParseOutcome` / `AdvisoryDiagnostic` | N/A (in-process) | docs/01,03 | source-driven owner + no feature promotion + compile-fail boundary + error-or-fatal/value exclusion |
| host/path admission | `HostAdmissionContext` / `BuildExecutionContext` / `ConfigResourceRoot` / `PortablePath` | portable path + config roots | docs/01,18,19 | ProjectRoot variant + containment + 0/1/>1 candidate result + no serialized HostPath |
| URI admission | `SafeUri` | typed URI fields | docs/03,15,18 | scheme/control/whitespace/length |
| length and transform | `Length` / `AffineTransform` | common defs | docs/24 | numeric/type checks |
| parser package | `ParsedPackage` | document root | docs/03,04 | Rust token + Schema |
| machine package ingestion | stable-byte admission, strict decoder, sealed source validation, and session-bound receipts are implemented; `WireDocumentPackage` remains untrusted | public 1.0/1.1/1.2 DocumentPackage input and current 1.2 output; target 1.3 adds trim/region/column/placement members but remains public-rejected | ADR-0027, ADR-0028, ADR-0031, docs/02,19,25,26, contracts/phase-ownership | current independent Schema/semantic validation and public E2E; future full 1.3 registry/typed round trip must land atomically at MI3-12 |
| machine PDF capability | exact public `PARAGRAPH_1`, `BASIC_DOCUMENT_1`, `FOOTNOTE_1`, and `TABLE_1` descriptors with matching preflight receipts; MI3-09/MI3-10 add private session/limits-bound header/footer and columns preflight receipts while all three ADR-0031 public IDs remain reserved | current 1.2 capability Schema and canonical four-profile fixture; future 1.3 adds conditional advanced descriptor members | contracts/machine-pdf-capabilities, ADR-0027, ADR-0028, ADR-0029, ADR-0030, ADR-0031, docs/26 | current bidirectional descriptor/fixture closure and release gates; future three combined fixtures/default/old-descriptor freeze at MI3-12 |
| basic-document profile | MI2-02 multi-flow owners, MI2-03 typed block-style receipts/consumers, MI2-04 syntax-owned marker/list receipts, MI2-05 forced-boundary receipts, MI2-06 admitted-PNG/figure-placement/DrawImage/XObject receipts, and MI2-07 package-bound link/cluster/rectangle/annotation receipts | current 1.2 DocumentPackage plus versioned multi-flow and selected block-style/list/forced-break/PNG-figure/link facts | ADR-0028, contracts/machine-pdf-capabilities, docs/25,26 | combined all-advertised fixture, typed closure, deterministic PDF goldens, exact limits, receipt swaps, and tamper negatives |
| table profile | MI3-02/MI3-03 resolved columns/grid, canonical cell FlowIds, row bands/fragments, rowspan continuation, and header repetition; MI3-04 exact Display-command and frozen-PDF graph closure | unchanged current 1.2 `table`/fixed/fraction/head/body/colspan/rowspan wire plus conditional table trace/manifest facts; no table-specific style field | ADR-0029, contracts/machine-pdf-capabilities, docs/10,25,26 | public `m3-table.json` table-only/combined coverage, exact limits, receipt and command tamper negatives, zero-decoration PDF/raster, reproducibility, and old-profile rejection gate |
| footnote profile | canonical FootnoteFlowIds, first-reference discovery, exact reservation/evaluation/convergence, dedicated carry-only pages, selected layout, definition-anchor/link paint, and Display/PDF closure | unchanged current 1.2 definition/reference/page-master-footnote wire plus conditional `machine-footnote-manifest` facts in trace/build manifest; no footnote-specific style field | ADR-0030, contracts/machine-pdf-capabilities, docs/04,08,09,10,25,26 | public `m3-footnote.json`: zero and combined M2, catalog-vs-first-reference order, repeat/split/multi-page carry, receipt/paint tamper, PDF/raster/text order, reproducibility, and old-profile rejection |
| advanced-pagination targets | MI3-09 implements checked master/trim/page boxes, canonical first/left/right selection, and independent header/footer flows; MI3-10 implements exact residual columns, canonical parent/source FlowIds, monotonic sequential fill, bounded final-page balance, and selected/Display/PDF/manifest closure. Both use session/limits-bound private receipts; float owners remain pending and no public Rust dispatch exists | private `schemas/1.3/` covers the advanced DocumentPackage additions and exact `machine-advanced-pagination-manifest`; current aliases and public output remain byte-frozen at 1.2 | ADR-0031, contracts/machine-pdf-capabilities, contracts/phase-ownership, docs/10,25 | MI3-09 and MI3-10 combined/empty/oversize private fixture gates with exact/max+1, oscillation, tamper, and reproducibility coverage; MI3-11 adds the float slice, then MI3-12 closes bidirectional fixtures, G6003/G6004, PDF-box/raster, old-profile rejection, and documented-host gates |
| canonical lists | current document list type plus package-bound marker-usage, item-flow layout, selected fragment, Display/PDF, and manifest receipts | ordered/start relation plus versioned 1.2 selected list facts | docs/04, ADR-0028, docs/25 | ordered positive start + checked item index, unordered null/U+2022, marker buffer/aggregate max+1, widest-column LTR/RTL placement, nested child-frame indent, marker orphan and missing/extra/wrong-item closure |
| forced page breaks | current typed `PageBreak` plus package/epoch/FlowId-bound layout boundary and exact consume receipt | versioned 1.2 selected forced-break cursor/page facts | ADR-0028, docs/09, docs/25 | start/middle/consecutive/trailing blank policy, `N + 1` pages, exact/max+1 page limit, stale-cursor and break-paint closure; `paragraph-1` remains closed |
| non-floating PNG figures | decoder-only `AdmittedImageMediaKind::Png`, package-bound Figure/caption usage, `ValidatedFigureLayout`, selected placement, one-DrawImage Display, finalized image/soft-mask plans, and graph/serializer-bound XObject facts | current 1.2 Figure plus versioned `machine-figure-manifest`; image declarations have no caller-authoritative media field | ADR-0028, docs/07,13,25 | opaque-suffix stable-read admission, full bounded decode, pixel/aspect dimensions, caption split/keep/terminal oversize, bad hash/non-PNG/invalid dimensions/pixel limit, missing/extra/wrong IDs and XObjects, publication failure, deterministic double build; `paragraph-1` remains closed |
| link annotations and named destinations | syntax-owned package anchor/`SafeUri` target receipt, selected logical shaping-cluster ranges, canonical page/line visual rectangle unions, Display annotations, and graph/serializer-bound PDF observations | current 1.2 links plus versioned `machine-link-manifest` | ADR-0028, docs/07,13,25 | wrapped internal/external links, scheme-only normalization, package-bound targets, nonempty/painted preflight, exact rectangle/object limits, missing/extra/wrong-page/wrong-target/rectangle closure, deterministic PDF golden; `paragraph-1` remains closed |
| block selectors/style cascade | current selector/cascade/`ResolvedTextStyle` plus the 1.2 eight-property registry and computed receipt | block classes + closed current 1.2 rules and exact tagged declarations | docs/04, ADR-0025, ADR-0028 | grammar/class order, tagged-value min/max/max+1, applicability, initial/inherit/extends/override, registry/owner/package binding, typed-consumer coverage |
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
| limits | `ResourceLimits` / `MachineInputLimitBounds` | config and capability Schemas | docs/03,07,09,10,18,19,25 | exact field set + package-byte/JSON-depth default/maximum identity + inclusive max semantics + iterative nesting precheck + `I9100`/`I9101`; ADR-0030 maps footnote work and ADR-0031 maps master/region/frame/balance/queue/carry work to existing limits, with target G6003/G6004 before max+1 |
| effective config/build manifest | `EffectiveConfig` / `BuildInputProfile` / `PackageInputRecord` / publication contexts | current config/build Schema; target 1.3 conditional `advanced_pagination` | docs/16,19,25, ADR-0031 | current raw 1.0/1.1/1.2 normalization and terminal publication; MI3-12 atomically adds raw/current 1.3, advanced profile/flow/selected/paint closure, and forbids the new member for old profiles |
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

ADR-0031 fixes the target mapping now; MI3-12 alone may move the “after”
column into public code. MI3-08 does not create or modify Schema files.

| Raw DocumentPackage contract | Profile | After MI3-08 / before MI3-12 | After the MI3-12 gate |
|---|---|---|---|
| 1.0 / 1.1 / 1.2 | omitted or `paragraph-1` | current frozen paragraph behavior; output is 1.2 | accepted with the same paragraph semantics; canonical output is 1.3 |
| neutral 1.3 paragraph subset | omitted or `paragraph-1` | public `P1103` because 1.3 is non-current | accepted only with full-media trim, null auxiliary content/columns, horizontal/LTR, and no Figure |
| non-neutral 1.3 advanced wire | omitted or `paragraph-1` | public `P1103` | `L5100`/`L5101`; default selection never upgrades a profile |
| 1.2 | `basic-document-1`, `table-1`, or `footnote-1` | accepted by the matching current profile | unchanged accepted semantic set; canonical output is current 1.3 |
| neutral 1.3 | `basic-document-1`, `table-1`, or `footnote-1` | public `P1103` because 1.3 is non-current | accepted as compatibility input with full-media trim, null region/columns, block Figures, and unchanged auxiliary-frame rules |
| non-neutral 1.3 | `basic-document-1`, `table-1`, or `footnote-1` | public `P1103` | `L5100`/`L5101`; no advanced feature is ignored, synthesized, or downgraded |
| 1.0 / 1.1 | `basic-document-1`, `table-1`, or `footnote-1` | existing old-contract rejection | `P1103` at `/contract`; typed 1.2 semantics are not synthesized |
| 1.3 | matching `header-footer-1`, `columns-1`, or `float-1` | profile is unknown usage exit 2 and contract is public `P1103` | accepted only for ADR-0031's profile-specific closed domain |
| 1.0 / 1.1 / 1.2 | any advanced profile | profile is unknown usage exit 2 | `P1103` at `/contract`; old packages are not upgraded into advanced pagination |
| unknown | any | `P1103` or unknown-profile usage | same; no newest-contract/profile fallback |

The default remains `typaxis.machine-pdf/paragraph-1`. At MI3-12,
`document_package_contracts` becomes exactly 1.0/1.1/1.2/1.3 and the canonical
profile order becomes `basic-document-1`, `columns-1`, `float-1`,
`footnote-1`, `header-footer-1`, `paragraph-1`, `table-1`. Existing profile
descriptor objects and table/footnote-specific projections omit
`advanced_pagination`; built new profiles require the identical canonical
selected record in trace and manifest. Neutral-1.3 compatibility preserves
the current-encoder `dump-ast -> build-package` path without expanding any old
profile's semantic domain.

The publication transaction validates the complete independent 1.3 registry,
then switches the current contract constant, Schema aliases, serializer,
decoder, `dump-ast`, normalized config, diagnostics, capability, trace,
manifest, public dispatch/help, and fixtures in one change set. It removes all
private runners in that same gate. The frozen 1.0, 1.1, and 1.2 registries are
never populated with 1.3 definitions.
