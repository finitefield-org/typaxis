# Cross-layer contract matrix

The evidence matrix below does not by itself report delivery completion. Machine input uses four independent status axes:

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| reference TSF build | Yes, current 1.2 | Yes, bounded reference subset | Yes | No |
| DocumentPackage portable validation/export | Yes, current 1.2 plus frozen 1.0/1.1 input | Yes: independent Schema registries/validator and shared `dump-ast` encoder | Yes, package commands and round trip | Yes |
| sealed package/source ingestion | Yes, ADR-0027 | Yes | Yes, macOS/Linux fixture gate | Yes, M1 host gate |
| `typaxis.machine-pdf/paragraph-1` | Yes, immutable capability contract | Yes | Yes, macOS/Linux combined PDF/sidecars | Yes |
| `typaxis.machine-pdf/basic-document-1` | Yes, ADR-0028 immutable M2 profile | Yes: canonical multi-flow, typed block-style, list, forced-page-break, PNG figure, and link annotation/named-destination pipeline | Yes, combined PDF/sidecars | Yes |
| `typaxis.machine-pdf/table-1` | Yes, ADR-0029 immutable M3 target on contract 1.2 | No: MI3-02/MI3-03 pending | No: unknown public profile until MI3-04 | No: MI3-04 gate |
| generated contract 1.2 artifacts | Yes | Yes: config/trace/diagnostics/manifest/package/capabilities/evidence | Yes | Yes |
| contract 1.2 publication | Yes, ADR-0028 migration table | Yes: current aliases plus the complete independent `schemas/1.2/` registry; former 1.1 is frozen | Yes | Yes |

`Contract-defined` does not imply that a Rust owner exists, the current `build` accepts DocumentPackage JSON, public CLI E2E passes, or a release supports the feature.

| Contract | Rust | JSON | Docs | Validator |
|---|---|---|---|---|
| product/CLI identity | `typaxis_core::PRODUCT_NAME` / Cargo `[[bin]]` | manifest `engine.name` | docs/19 | exact name/bin/Schema checks |
| wire ID | current `typaxis_core::CONTRACT` is 1.2; typed DocumentPackage input IDs are 1.0/1.1/1.2 | current roots use 1.2; complete 1.0 and 1.1 registries are frozen separately | contract-version, ADR-0027, ADR-0028, ADR-0029 | independent frozen 1.0/1.1 and current/versioned 1.2 registries, compatibility hashes, no cross-registration, and unchanged 1.2 table wire bytes |
| source/text/local map range | `SourceSpan` / `TextSpan` / `Utf8ByteRange` | common + document package | docs/03 | bounds/boundary/coverage |
| generated/Display text ownership | `GeneratedBufferKey` / `GeneratedTextStore` / `DisplayTextMap` / `DisplayDocument.text_buffers` | display text buffers/spans | docs/05,09,11 | canonical key allocation + disjoint internal IDs + selected-bound stable dense remap + artifact-owned text table |
| validated parser output | sealed `Parser` / `ValidatedParsedPackage` / `ParseOutcome` / `AdvisoryDiagnostic` | N/A (in-process) | docs/01,03 | source-driven owner + no feature promotion + compile-fail boundary + error-or-fatal/value exclusion |
| host/path admission | `HostAdmissionContext` / `BuildExecutionContext` / `ConfigResourceRoot` / `PortablePath` | portable path + config roots | docs/01,18,19 | ProjectRoot variant + containment + 0/1/>1 candidate result + no serialized HostPath |
| URI admission | `SafeUri` | typed URI fields | docs/03,15,18 | scheme/control/whitespace/length |
| length and transform | `Length` / `AffineTransform` | common defs | docs/24 | numeric/type checks |
| parser package | `ParsedPackage` | document root | docs/03,04 | Rust token + Schema |
| machine package ingestion | stable-byte admission, strict decoder, sealed source validation, and session-bound receipts are implemented; `WireDocumentPackage` remains untrusted | 1.0/1.1/1.2 DocumentPackage input; current output is 1.2 | ADR-0027, ADR-0028, docs/02,19,25,26, contracts/phase-ownership | independent Schema/semantic validation, Rust receipt tests, public positive/negative package-command E2E |
| machine PDF capability | exact `PARAGRAPH_1` and `BASIC_DOCUMENT_1` descriptors with matching preflight receipts | current 1.2 capability Schema and canonical fixture | contracts/machine-pdf-capabilities, ADR-0027, ADR-0028, docs/26 | bidirectional descriptor/fixture closure, compatibility/default goldens, public combined E2E, external PDF and cross-checkout reproducibility gates |
| basic-document profile | MI2-02 multi-flow owners, MI2-03 typed block-style receipts/consumers, MI2-04 syntax-owned marker/list receipts, MI2-05 forced-boundary receipts, MI2-06 admitted-PNG/figure-placement/DrawImage/XObject receipts, and MI2-07 package-bound link/cluster/rectangle/annotation receipts | current 1.2 DocumentPackage plus versioned multi-flow and selected block-style/list/forced-break/PNG-figure/link facts | ADR-0028, contracts/machine-pdf-capabilities, docs/25,26 | combined all-advertised fixture, typed closure, deterministic PDF goldens, exact limits, receipt swaps, and tamper negatives |
| table profile target | pending MI3-02/MI3-03 owners: resolved columns/grid, canonical cell FlowIds, row bands/fragments, rowspan continuation, and header repetition; pending MI3-04 Display/PDF closure | unchanged current 1.2 `table`/fixed/fraction/head/body/colspan/rowspan wire; no table-specific style field | ADR-0029, contracts/machine-pdf-capabilities, docs/10,25 | contract-defined only: future `m3-table.json` combined coverage, exact limits, receipt tamper, zero-decoration PDF/raster, reproducibility, and old-profile rejection gate |
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
| limits | `ResourceLimits` / `MachineInputLimitBounds` | config and capability Schemas | docs/03,07,09,10,18,19,25 | exact field set + package-byte/JSON-depth default/maximum identity + inclusive max semantics + iterative nesting precheck + `I9100`/`I9101` mapping |
| effective config/build manifest | `EffectiveConfig` / `BuildInputProfile` / `PackageInputRecord` / publication contexts | config + build Schema | docs/16,19,25 | precedence/canonical JCS hash + raw 1.0/1.1/1.2 normalization + reference/machine input conditional + profile/flow receipt closure + package byte limit + terminal publication |
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

ADR-0029 fixes table semantics without migrating the current wire. Contract
definition and public support remain separate until MI3-04.

| Raw DocumentPackage contract | Profile | After MI3-01 / before MI3-04 | After the MI3-04 gate |
|---|---|---|---|
| 1.0 / 1.1 | `table-1` | unknown profile usage exit 2 | `P1103` at `/contract`; no style/profile upgrade |
| 1.2 | `paragraph-1` or `basic-document-1` with table | existing `L5100` rejection | unchanged `L5100` rejection |
| 1.2 | `table-1` | contract-defined only; public CLI rejects the profile while private slices remain non-public | accepted only for ADR-0029's closed direct-body table domain |
| 1.2 with a table border/padding/alignment/background/split field | `table-1` | `P1102` as unknown current wire/style | unchanged; requires a new contract and profile |
| unknown | any | `P1103` or unknown-profile usage error | same; no newest-contract/profile fallback |

MI3-04 may move only the `table-1` status axes after descriptor/combined-fixture
bidirectional coverage, grid/rowspan/header/Display/PDF receipt closure,
inclusive `max_ast_nodes`/`max_fragments` max+1 checks, zero-decoration raster
evidence, reproducibility, and older-profile rejection goldens all pass. It does
not change the current contract ID, DocumentPackage Schema bytes, or default
profile.
