# Cross-layer contract matrix

The evidence matrix below does not by itself report delivery completion. Machine input uses four independent status axes:

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| reference TSF build | Yes, current 1.1 | Yes, bounded reference subset | Yes | No |
| DocumentPackage portable validation/export | Yes, current 1.1 plus frozen 1.0 input | Yes: dual Schema/validator and shared `dump-ast` encoder | Yes, package commands and round trip | Yes, M1 host gate |
| sealed package/source ingestion | Yes, ADR-0027 | Yes | Yes, macOS/Linux fixture gate | Yes, M1 host gate |
| `typaxis.machine-pdf/paragraph-1` | Yes, immutable capability contract | Yes | Yes, macOS/Linux combined PDF/sidecars | Yes |
| `typaxis.machine-pdf/basic-document-1` | Yes, ADR-0028 immutable M2 target | No, M2 slices pending | No, public CLI rejects it until MI2-08 | No |
| generated contract 1.1 artifacts | Yes | Yes: config/trace/diagnostics/manifest/package/capabilities/evidence | Yes | Yes, M1 host gate |
| contract 1.2 staging/publication | Yes, ADR-0028 reservation and migration table | No versioned staging Schema yet | No; current aliases/output stay 1.1 | No; MI2-08 is the atomic gate |

`Contract-defined` does not imply that a Rust owner exists, the current `build` accepts DocumentPackage JSON, public CLI E2E passes, or a release supports the feature.

| Contract | Rust | JSON | Docs | Validator |
|---|---|---|---|---|
| product/CLI identity | `typaxis_core::PRODUCT_NAME` / Cargo `[[bin]]` | manifest `engine.name` | docs/19 | exact name/bin/Schema checks |
| wire ID | current `typaxis_core::CONTRACT` is 1.1; typed DocumentPackage input IDs are 1.0/1.1 | current roots use 1.1; the seven-schema 1.0 registry is frozen separately | contract-version, ADR-0027 | independent frozen seven-schema 1.0/current eleven-schema 1.1 registries, compatibility hash, and no cross-registration |
| source/text/local map range | `SourceSpan` / `TextSpan` / `Utf8ByteRange` | common + document package | docs/03 | bounds/boundary/coverage |
| generated/Display text ownership | `GeneratedBufferKey` / `GeneratedTextStore` / `DisplayTextMap` / `DisplayDocument.text_buffers` | display text buffers/spans | docs/05,09,11 | canonical key allocation + disjoint internal IDs + selected-bound stable dense remap + artifact-owned text table |
| validated parser output | sealed `Parser` / `ValidatedParsedPackage` / `ParseOutcome` / `AdvisoryDiagnostic` | N/A (in-process) | docs/01,03 | source-driven owner + no feature promotion + compile-fail boundary + error-or-fatal/value exclusion |
| host/path admission | `HostAdmissionContext` / `BuildExecutionContext` / `ConfigResourceRoot` / `PortablePath` | portable path + config roots | docs/01,18,19 | ProjectRoot variant + containment + 0/1/>1 candidate result + no serialized HostPath |
| URI admission | `SafeUri` | typed URI fields | docs/03,15,18 | scheme/control/whitespace/length |
| length and transform | `Length` / `AffineTransform` | common defs | docs/24 | numeric/type checks |
| parser package | `ParsedPackage` | document root | docs/03,04 | Rust token + Schema |
| machine package ingestion | stable-byte admission, strict decoder, sealed source validation, and session-bound receipts are implemented; `WireDocumentPackage` remains untrusted | 1.0/1.1 DocumentPackage input; current output is 1.1 | ADR-0027, docs/02,19,25,26, contracts/phase-ownership | dual Schema/semantic validation, Rust receipt tests, public positive/negative package-command E2E |
| machine PDF capability | `MachineProfileDescriptor::PARAGRAPH_1` and matching preflight receipt | current 1.1 capability Schema and canonical fixture | contracts/machine-pdf-capabilities, ADR-0027, docs/26 | descriptor/encoder exact fixture, invalid limit fixture, public combined E2E, M2-negative assertion, external PDF/reproducibility gates, completed macOS/Linux aggregation |
| basic-document target | future `BasicDocumentPreflightReceipt`, `ValidatedFlowRegistryReceipt`, and typed style/figure/link owners | reserved 1.2 versioned Schema IDs; no current alias before MI2-08 | ADR-0028, contracts/machine-pdf-capabilities, docs/25 | closed block/inline/style/resource set; blank-page/keep/oversize/URI policy; existing-limit mapping; full body/subflow trace/manifest closure; public negative assertion until atomic publication |
| canonical lists | document list type | ordered/start relation | docs/04 | ordered positive start + unordered null |
| block selectors/style cascade | style selector/cascade/`ResolvedTextStyle` types | block classes + closed typed style rules | docs/04 | grammar/class order/property registry/required text style/extends/winner |
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
| effective config/build manifest | `EffectiveConfig` / `BuildInputProfile` / `PackageInputRecord` / publication contexts | config + build Schema | docs/16,19,25 | precedence/canonical JCS hash + raw 1.0 normalization + reference/machine input conditional + package byte limit + terminal publication |
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

The default remains `typaxis.machine-pdf/paragraph-1`. MI2-02 through MI2-07 may validate versioned 1.2 staging artifacts only through the crate-private runner. MI2-08 freezes 1.1, switches every current artifact/decoder/`dump-ast` alias to 1.2, removes staging access, publishes the descriptor and combined fixture, and only then changes the public/release status rows above.
