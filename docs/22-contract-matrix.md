# Cross-layer contract matrix

The evidence matrix below does not by itself report delivery completion. Machine input uses four independent status axes:

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| reference TSF build | Yes, current 1.3 | Yes, bounded reference subset | Yes | No |
| DocumentPackage portable validation/export | Yes, current 1.3 plus frozen 1.0/1.1/1.2 input | Yes: independent Schema registries/validator and shared `dump-ast` encoder | Yes, package commands and round trip | Yes |
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
| contract 1.3 publication | Yes, ADR-0031 | Yes: current aliases, complete independent registry, and seven public profiles | Yes | Yes, MI3-12 gate |
| contract 1.4 / `production-book-1` target | Yes, ADR-0032 base decision | No: private staging begins MI4-02 | No; current aliases remain 1.3 | No, MI4-13 gate |

`Contract-defined` does not imply that a Rust owner exists, the current `build` accepts DocumentPackage JSON, public CLI E2E passes, or a release supports the feature.

| Contract | Rust | JSON | Docs | Validator |
|---|---|---|---|---|
| product/CLI identity | `typaxis_core::PRODUCT_NAME` / Cargo `[[bin]]` | manifest `engine.name` | docs/19 | exact name/bin/Schema checks |
| wire ID | current `typaxis_core::CONTRACT` is 1.3; typed public DocumentPackage input IDs are 1.0/1.1/1.2/1.3; ADR-0032 reserves non-current 1.4 | current roots use 1.3; complete 1.0, 1.1, and 1.2 registries are frozen separately; `schemas/1.3/` is the complete current registry; no 1.4 Schema exists at MI4-01 | contract-version, ADR-0027 through ADR-0032 | independent frozen 1.0/1.1/1.2 and current/versioned 1.3 registries now; MI4-13 must validate a complete independent 1.4 registry before switching aliases |
| source/text/local map range | `SourceSpan` / `TextSpan` / `Utf8ByteRange` | common + document package | docs/03 | bounds/boundary/coverage |
| generated/Display text ownership | `GeneratedBufferKey` / `GeneratedTextStore` / `DisplayTextMap` / `DisplayDocument.text_buffers` | display text buffers/spans | docs/05,09,11 | canonical key allocation + disjoint internal IDs + selected-bound stable dense remap + artifact-owned text table |
| validated parser output | sealed `Parser` / `ValidatedParsedPackage` / `ParseOutcome` / `AdvisoryDiagnostic` | N/A (in-process) | docs/01,03 | source-driven owner + no feature promotion + compile-fail boundary + error-or-fatal/value exclusion |
| host/path admission | `HostAdmissionContext` / `BuildExecutionContext` / `ConfigResourceRoot` / `PortablePath` | portable path + config roots | docs/01,18,19 | ProjectRoot variant + containment + 0/1/>1 candidate result + no serialized HostPath |
| URI admission | `SafeUri` | typed URI fields | docs/03,15,18 | scheme/control/whitespace/length |
| length and transform | `Length` / `AffineTransform` | common defs | docs/24 | numeric/type checks |
| parser package | `ParsedPackage` | document root | docs/03,04 | Rust token + Schema |
| machine package ingestion | stable-byte admission, strict decoder, sealed source validation, and session-bound receipts are implemented; `WireDocumentPackage` remains untrusted; target 1.4 adds a closed semantic-container block and required declared media | public 1.0/1.1/1.2/1.3 DocumentPackage input and current 1.3 output; target 1.4 remains public `P1103` | ADR-0027, ADR-0028, ADR-0031, ADR-0032, docs/02,19,25,26, contracts/phase-ownership | current independent Schema/semantic validation, typed round trip, and public E2E; MI4-02 starts private 1.4 and MI4-13 is the atomic gate |
| machine PDF capability | exact seven-profile descriptors with matching preflight receipts; ADR-0032 reserves no public descriptor yet | current 1.3 capability Schema and canonical seven-profile fixture; target 1.4 reserves `production-book-1` | contracts/machine-pdf-capabilities, ADR-0027 through ADR-0032, docs/26 | current bidirectional descriptor/combined-fixture closure and `m3-all.json`; future M4 combined fixture/default/old-profile freeze at MI4-13 |
| basic-document profile | MI2-02 multi-flow owners, MI2-03 typed block-style receipts/consumers, MI2-04 syntax-owned marker/list receipts, MI2-05 forced-boundary receipts, MI2-06 admitted-PNG/figure-placement/DrawImage/XObject receipts, and MI2-07 package-bound link/cluster/rectangle/annotation receipts | frozen 1.2 DocumentPackage semantics plus their exact neutral 1.3 encoding and versioned selected facts | ADR-0028, contracts/machine-pdf-capabilities, docs/25,26 | combined all-advertised fixture, typed closure, deterministic PDF goldens, exact limits, receipt swaps, and tamper negatives |
| table profile | MI3-02/MI3-03 resolved columns/grid, canonical cell FlowIds, row bands/fragments, rowspan continuation, and header repetition; MI3-04 exact Display-command and frozen-PDF graph closure | unchanged 1.2 `table` wire or neutral 1.3 encoding plus conditional table trace/manifest facts; no table-specific style field | ADR-0029, contracts/machine-pdf-capabilities, docs/10,25,26 | public `m3-table.json` table-only/combined coverage, exact limits, receipt and command tamper negatives, zero-decoration PDF/raster, reproducibility, and old-profile rejection gate |
| footnote profile | canonical FootnoteFlowIds, first-reference discovery, exact reservation/evaluation/convergence, dedicated carry-only pages, selected layout, definition-anchor/link paint, and Display/PDF closure | unchanged 1.2 definition/reference/page-master-footnote wire or neutral 1.3 encoding plus conditional `machine-footnote-manifest` facts; no footnote-specific style field | ADR-0030, contracts/machine-pdf-capabilities, docs/04,08,09,10,25,26 | public `m3-footnote.json`: zero and combined M2, catalog-vs-first-reference order, repeat/split/multi-page carry, receipt/paint tamper, PDF/raster/text order, reproducibility, and old-profile rejection |
| advanced-pagination profiles | MI3-09 implements checked master/trim/page boxes, canonical first/left/right selection, and independent header/footer flows; MI3-10 implements exact residual columns, canonical parent/source FlowIds, monotonic sequential fill, bounded final-page balance, and selected/Display/PDF/manifest closure; MI3-11 implements canonical FIFO float anchors, typed here/top/bottom/next-page decisions, nonwrapping exclusion bands, bounded page carry, and placement/object closure; MI3-12 binds admitted content/resources and exposes normal public dispatch | current `schemas/1.3/` covers the advanced DocumentPackage additions and exact conditional `machine-advanced-pagination-manifest` | ADR-0031, contracts/machine-pdf-capabilities, contracts/phase-ownership, docs/10,25,26 | three all-advertised combined fixtures, exact/max+1 and progress gates, public G6003/G6004, PDF boxes/raster/text, reproducibility, old-profile freeze, and aggregate `m3-all.json` |
| semantic-container target | target owner graph fixes closed `result`, `proof`, `exercise`, NodeId/SourceSpan/child ownership, `semantic_container` style scope, one canonical container FlowId, strict page-fragment progress, and one outline/tag structure boundary | target 1.4 block requires `kind`, `node_id`, `span`, `classes`, `semantic_kind`, and nonempty `blocks`; current/frozen Schemas remain unchanged at MI4-01 | ADR-0032, contracts/machine-pdf-capabilities, contracts/phase-ownership, docs/25 | MI4-02 private result/proof/exercise, nested/split/empty/unknown/wrong-owner/round-trip/tamper fixtures; MI4-13 public combined closure |
| declared-media target | target domain separates provenance-bound `LegacyUnspecified` from typed `Declared`; machine profile owns allowed set; resource admission alone issues and exact-matches attestation; source exporter consumes that same receipt | target 1.4 requires image `png` and fonts `sfnt-truetype-glyf`, `ttc-truetype-glyf`; M4 manifest adds tagged declaration and M4 font attestation while frozen old images retain their existing PNG attestation | ADR-0032, contracts/contract-version, contracts/phase-ownership, docs/25 | MI4-02 base round-trip/mismatch and source-export tests; MI4-03/MI4-10 add only assigned values; MI4-13 freezes old bytes and M4 success/failure branches |
| canonical lists | current document list type plus package-bound marker-usage, item-flow layout, selected fragment, Display/PDF, and manifest receipts | ordered/start relation plus versioned 1.2 selected list facts | docs/04, ADR-0028, docs/25 | ordered positive start + checked item index, unordered null/U+2022, marker buffer/aggregate max+1, widest-column LTR/RTL placement, nested child-frame indent, marker orphan and missing/extra/wrong-item closure |
| forced page breaks | current typed `PageBreak` plus package/epoch/FlowId-bound layout boundary and exact consume receipt | versioned 1.2 selected forced-break cursor/page facts | ADR-0028, docs/09, docs/25 | start/middle/consecutive/trailing blank policy, `N + 1` pages, exact/max+1 page limit, stale-cursor and break-paint closure; `paragraph-1` remains closed |
| non-floating PNG figures | decoder-only `AdmittedImageMediaKind::Png`, package-bound Figure/caption usage, `ValidatedFigureLayout`, selected placement, one-DrawImage Display, finalized image/soft-mask plans, and graph/serializer-bound XObject facts | frozen 1.2 Figure or required 1.3 `placement=block` plus versioned `machine-figure-manifest`; current declarations have no caller-authoritative media field, while target 1.4 requires `media_type=png` without treating it as attestation | ADR-0028, ADR-0032, docs/07,13,25 | opaque-suffix stable-read admission, full bounded decode, pixel/aspect dimensions, caption split/keep/terminal oversize, bad hash/non-PNG/invalid dimensions/pixel limit, missing/extra/wrong IDs and XObjects, publication failure, deterministic double build; target declaration/mismatch stays private until M4 publication |
| link annotations and named destinations | syntax-owned package anchor/`SafeUri` target receipt, selected logical shaping-cluster ranges, canonical page/line visual rectangle unions, Display annotations, and graph/serializer-bound PDF observations | unchanged 1.2 link wire carried into 1.3 plus versioned `machine-link-manifest` | ADR-0028, docs/07,13,25 | wrapped internal/external links, scheme-only normalization, package-bound targets, nonempty/painted preflight, exact rectangle/object limits, missing/extra/wrong-page/wrong-target/rectangle closure, deterministic PDF golden; `paragraph-1` remains closed |
| block selectors/style cascade | current selector/cascade/`ResolvedTextStyle` plus the frozen 1.2 eight-property registry and computed receipt | block classes plus unchanged tagged declarations in current 1.3 | docs/04, ADR-0025, ADR-0028 | grammar/class order, tagged-value min/max/max+1, applicability, initial/inherit/extends/override, registry/owner/package binding, typed-consumer coverage |
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
| effective config/build manifest | `EffectiveConfig` / `BuildInputProfile` / `PackageInputRecord` / publication contexts | current 1.3 config/build Schema with conditional `advanced_pagination`; target 1.4 production-resource branch adds declaration/attestation only | docs/16,19,25, ADR-0031, ADR-0032 | current raw 1.0/1.1/1.2/1.3 normalization and terminal publication; MI4-13 must preserve old raw-contract/profile 1.3 bytes while atomically adding 1.4 version dispatch and forbidding M4 fields in old-profile branches |
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
| 1.2 | `basic-document-1`, `table-1`, or `footnote-1` | accepted by the matching current profile | unchanged accepted semantic set; canonical output is current 1.3 |
| neutral 1.3 | `basic-document-1`, `table-1`, or `footnote-1` | public `P1103` because 1.3 is non-current | accepted as compatibility input with full-media trim, null region/columns, block Figures, and unchanged auxiliary-frame rules |
| non-neutral 1.3 | `basic-document-1`, `table-1`, or `footnote-1` | public `P1103` | `L5100`/`L5101`; no advanced feature is ignored, synthesized, or downgraded |
| 1.0 / 1.1 | `basic-document-1`, `table-1`, or `footnote-1` | existing old-contract rejection | `P1103` at `/contract`; typed 1.2 semantics are not synthesized |
| 1.3 | matching `header-footer-1`, `columns-1`, or `float-1` | profile is unknown usage exit 2 and contract is public `P1103` | accepted only for ADR-0031's profile-specific closed domain |
| 1.0 / 1.1 / 1.2 | any advanced profile | profile is unknown usage exit 2 | `P1103` at `/contract`; old packages are not upgraded into advanced pagination |
| unknown | any | `P1103` or unknown-profile usage | same; no newest-contract/profile fallback |

The default remains `typaxis.machine-pdf/paragraph-1`.
`document_package_contracts` is exactly 1.0/1.1/1.2/1.3 and the canonical
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

## M4 semantic-container and declared-media migration matrix

ADR-0032 fixes the base mapping now. MI4-01 creates no Schema or Rust surface;
MI4-02 through MI4-12 may implement only private contract-1.4 staging, and
MI4-13 alone may move the “after” column into public code.

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
`result|proof|exercise` kind and a nonempty block list. It is block-only, owns
one canonical FlowId, preserves one typed wrapper and the
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

Only the 1.4 production-manifest branch adds tagged `media_declaration` and M4
font attestation. Built M4 records require declared/non-null exact match.
Current image records already have required PNG `attested_media_kind`; frozen
old image/font shapes remain unchanged. Legacy/null is limited to an
old-contract M4 request rejected before resource admission. Existing old
raw-contract/profile artifacts stay on the frozen 1.3 registry; raw 1.4 or any
M4 profile request uses 1.4 version dispatch.

MI4-13 must validate the complete independent 1.4 registry, then atomically
switch/register contract decode, versioned encode, resource-attested
`dump-ast`, config, diagnostics, manifest dispatch, current Schema aliases,
capability/profile/help, fixtures, and evidence. The default remains
`paragraph-1`; the canonical profile order gains `production-book-1` only at
that gate. Raw 1.4 requires explicit production-profile selection, including
for a source-export round trip; no old profile gains contract 1.4. No 1.4
definition is added to frozen 1.0 through 1.3 registries.
