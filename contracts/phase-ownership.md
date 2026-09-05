# Phase ownership

## Implemented public machine-input ownership

The following rows are the ownership adopted by [ADR-0027](../adr/ADR-0027-machine-document-package-ingestion.md), extended through [ADR-0031](../adr/ADR-0031-advanced-pagination-profiles.md), and completed for M4 by [ADR-0032](../adr/ADR-0032-semantic-container-and-declared-media.md) through [ADR-0037](../adr/ADR-0037-producer-composed-math-vector.md). Public package commands and local host evidence cover all eight immutable profiles. The lower table remains the shared base ownership inventory rather than an alternative machine-input path.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| compiled contained-package/contained-resource/atomic-publish availability tokens | machine-input, resource-admission, and atomic-publisher owners respectively; composed by `typaxis-machine-profile` | drive `profiles[].available`; contained-open tokens drive PACKAGE-before-read `I9110`, while missing atomic publication fails during context construction; CLI does not duplicate booleans |
| fixed `MAX_RESOURCE_ROOTS` and `MAX_HOST_READ_CANDIDATES` | `typaxis-host-admission` | preflight before root identity/open and candidate open; capability JSON projects the same constants |
| package/resource root handles, contained component walk, same-handle snapshot, bounded stable bytes, and host read/write identity ledger | `typaxis-host-admission` | issue generic session-bound host receipts only; never infer logical IDs or canonical records |
| PACKAGE HostPath/default or explicit package-root resolution and root-relative package URI | `typaxis-machine-input` using host-admission receipts | bind one machine admission session; serialize only `PortablePath`, never absolute root/path |
| strict JSON lexical preflight, caller-constructible `WireDocumentPackage`, decoder-issued `DecodedDocumentPackage`, JSON location index, and package JCS hash | `typaxis-document-package` | portable decode/export only; never issue host or trusted syntax authority |
| raw PACKAGE receipt, decoded binding, exact single companion-source set, read budgets, and monotonic machine-input progress | `typaxis-machine-input` | issue `AdmittedMachinePackage`; reject cross-session raw/decoded/source receipt substitution |
| DTO lowering, actual source/TextMap/AST/style/master/resource validation, entry-only closure, and trusted package issuance | sealed `typaxis-syntax::DocumentPackageParser` | issue `ValidatedMachinePackage { ValidatedParsedPackage, provenance }`; no public DTO promotion path |
| eight immutable public descriptors, host availability, deterministic preflight order, and capability receipt | `typaxis-machine-profile` | generate canonical capability JSON and require the same profile/package/style/session binding at machine layout entry |
| all safe declared resource candidates before capability preflight | `typaxis-host-admission::HostReadIdentityLedger`, populated by machine orchestration | prevent requested diagnostics/manifest/PDF targets from aliasing existing or missing input candidates without opening resource bytes |
| logical font/image binding, bytes-derived metadata, partial progress, and complete resource ledger | `typaxis-resource-admission` using generic host receipts | capability-success path only; layout/finalization receives complete ledger, failed manifest may receive sealed progress |
| typed phase diagnostic code/category/subject and command-wide 256-record budget | originating phase plus `typaxis-diagnostics` materializer | map subjects through package JSON/source indexes; stderr and canonical sidecar read the same typed diagnostic, never parse `Debug` strings |
| raw/canonical package facts, source/resource progress, profile receipt, layout/PDF facts, and canonical manifest projection | `typaxis-manifest` owned-facts factory | accept only monotonic sealed progress/complete receipts; caller cannot author record fields as trusted facts |
| option/config/target validation and the fixed phase sequence | `typaxis-cli` orchestration | call owners single-directionally; source loader and package loader remain distinct |
| diagnostics -> failed manifest and trace -> PDF -> diagnostics -> built manifest visible order | self-consuming terminal publication owner | each file is individually atomic; retain exact partial visibility and never claim a multi-file transaction/rollback |

Machine progress is monotonic and owner-issued:

```text
NoInput
  -> RawPackageAdmitted
  -> PackageDecoded
  -> SourcesAdmitted
  -> PackageValidated
  -> CapabilityValidated
  -> ResourcesAdmitted
  -> LayoutSelected
```

`typaxis-machine-input -> typaxis-syntax` is forbidden. `WireDocumentPackage` is explicitly untrusted; decoder-issued `DecodedDocumentPackage`, session-bound package/source receipts, `ValidatedMachinePackage`, capability receipts, and publication receipts have private fields, no public raw-parts constructor, and no `Clone`. A downstream owner may project only the last issued token and must not recreate upstream facts from a DTO, error message, path, or canonical artifact.

## Implemented M2 basic-document ownership

These rows are the public contract 1.2 ownership adopted by [ADR-0028](../adr/ADR-0028-basic-document-profile.md) and integrated by MI2-08. Focused slice types retain their historical `Staging` names for source compatibility, but no dedicated staging runner or hidden profile selector is a production entry point.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| closed `basic-document-1` feature/policy table and `typaxis.basic-profile-receipt/1` fingerprint | `typaxis-machine-profile` | derive the public descriptor, typed preflight, capability projection, fixture coverage, and `BasicDocumentPreflightReceipt` |
| contract 1.2 declaration enum and exact tagged-value relation | `typaxis-document-package` versioned 1.2 decoder/encoder | issue typed wire values only; unknown/wrong tagged values are `P1102` and raw strings never reach style/layout |
| initial/inherit/cascade/applicability registry for 1.2 block properties | `typaxis-style` | issue package/style/registry-version-bound computed-style receipts; inapplicable known properties are `L5101` |
| canonical body/list-item/caption flow allocation, owner/parent/terminal closure, and `typaxis.basic-flow-registry/1` hash | `ProductionFlowIrBuilder` / `ValidatedFlowContentRegistry` in the layout-contract owner | issue `ValidatedFlowRegistryReceipt`; caller insertion and worker completion order cannot assign FlowId |
| checked list marker bytes and marker/item-first-paint keep group | generated-text owner and layout fragment owner respectively | bind marker buffer/limits and prevent marker orphaning |
| typed forced-boundary consume and before/after cursor progress | pagination | produce the ADR-0028 blank-page result and reject a repeated cursor as `I9190`; Display emits no break paint |
| PNG media attestation, dimensions, decoded-byte accounting, and ImageResourceId | resource-admission PNG decoder | feed figure geometry/finalization; URI suffix and caller media strings have no authority |
| figure width/aspect geometry, caption FlowId, keep and oversize outcome | typed style/layout figure owner | issue one selected figure placement consumed by Display, finalization, PDF, and manifest |
| link logical cluster range and selected page/line rectangle union | itemizer then selected-layout link owner | issue canonical nonempty rectangles; Display/PDF cannot infer links from coordinates or raw URI text |
| selected body/subflow state and full basic-document artifact closure | selected-state owner | bind preflight, registry, flow cursors, breaks, markers, figures, links, and resource ledger into trace/Display/PDF/manifest |
| frozen 1.2/1.3 registries and current 1.4 registry | Schema/contract integration owner | keep every version independent; current aliases, decoder, help, capabilities, fixtures, and generated artifacts switch only at an atomic publication milestone |

The M2 progress suffix is `ResourcesAdmitted -> FlowRegistryValidated -> LayoutSelected`. A downstream phase cannot reconstruct a flow registry from trace JSON, infer PNG from a path, normalize a URI again, relax keep/oversize policy, or fabricate profile/registry hashes for manifest output.

## Implemented M3 table ownership

These rows are the public table contract adopted by ADR-0029 and integrated by
MI3-04. They add a direct-body table to the complete M2 domain without changing
contract 1.2 bytes or either older profile.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| closed `table-1` domain, fixed zero-decoration policy, and `typaxis.table-profile-receipt/1` fingerprint | `typaxis-machine-profile` | derive the public descriptor, typed preflight, capability projection, and table fixture coverage |
| fixed/fraction resolution, signed residual recipient, dense grid, cell origins/spans, and canonical cell FlowIds | table grid/layout owner | issue `ValidatedTableGridReceipt`; caller order and Display coordinates cannot assign columns or cell ownership |
| cell paragraph fragments, row bands, common cuts, rowspan continuation, and header repetition | layout/pagination table owners | issue the complete `SelectedTableLayoutReceipt` with strict cursor progress and original-header bindings |
| exact cell rectangles, canonical cell glyph commands, and zero table decoration | table Display owner | issue `TableDisplayClosureReceipt`; missing/extra/wrong-cell/page/repetition commands or any path decoration fail before PDF publication |
| retained table commands in the frozen graph and serialized page streams | PDF graph and serializer owners | reopen the actual Display/PDF observations and bind them to the same table closure instead of trusting caller counts |
| selected grid/row/cell/header facts in trace and built manifest | selected-table and manifest projection owners | require identical `table_layouts` closure for built `table-1`; omit the member for older profiles to preserve their artifact bytes |

The M3 progress suffix remains `ResourcesAdmitted -> FlowRegistryValidated ->
LayoutSelected -> DisplayClosed -> PdfGraphFrozen`. Trace or manifest JSON is
never an authority for reconstructing a grid, repetition, command, or PDF
observation.

## Public M3 footnote ownership

These rows are the ownership adopted by
[ADR-0030](../adr/ADR-0030-footnote-profile.md), implemented by MI3-06, and
published end to end by MI3-07.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| closed `footnote-1` domain; catalog-ordinal marker, fixed separator, `allow` split, frame, and convergence policies; `typaxis.footnote-profile-receipt/1` | `typaxis-machine-profile` footnote descriptor/preflight owner | derive public capability, typed preflight, and fixture coverage from one descriptor |
| definition/reference/unreferenced closure and one FootnoteFlowId per canonical FootnoteId-owned definition | syntax profile preflight then canonical flow-registry owner | reject missing/duplicate/unreferenced/empty/nested definitions before allocation; issue `typaxis.footnote-flow-registry/1` independent of caller and first-reference order |
| initial marker ordinal/bytes and reference/definition style ownership | validated-package generated-text owner | bind every reference site and one definition site to the same catalog-derived ASCII decimal marker without pagination-dependent renumbering |
| selected body reference occurrences, page-local first-reference deduplication, and dense global assignment ordinals | page footnote discovery owner under pagination | place incoming carry before new assignments; repeated references remain observed but cannot clone an assignment |
| minimum-first definition fragmentation, exact reservation, fixed separator geometry, and reduced body frame | footnote layout/reservation owner | issue candidate-scoped fragmentation/reservation receipts without reconstructing geometry from paint or trace |
| `typaxis.footnote-page-evaluation/1` tuple, inclusive reflow consumption, consecutive equality, oscillation/exhaustion outcome | pagination work-budget and convergence owner | issue a converged page only; refuse `G6002` before max+1 and prevent any unmaterialized page from reaching Display/PDF |
| FootnoteFlowId/source-page/next-page/strictly advancing cursor carry | dedicated footnote carry owner | transport unfinished definitions independently of the body cursor; reject missing/duplicate/reordered/nonadvancing carry as `I9190` |
| body/reference, separator, definition fragment, and carry selected-state closure | selected-footnote-layout owner | issue `typaxis.footnote-selected-layout/1` covering every reference marker and every referenced definition's logical content exactly once |
| canonical separator and marker/definition commands | footnote Display owner | issue `typaxis.footnote-paint-closure/1`; body paints first, then one separator and ordered note fragments; no caller-authored coordinates or inferred continuation text |
| retained footnote commands/observations and selected trace/manifest projection | PDF graph/serializer and manifest owned-facts owners | bind body fingerprint, ordered assignments, reservation, evaluation count, fragments, carries, and paint hashes; the ID-sorted page projection alone is not authority |

The public progress suffix is `ResourcesAdmitted -> FootnoteFlowRegistryValidated
-> PageFootnoteConverged -> LayoutSelected -> DisplayClosed -> PdfGraphFrozen`.
Marker catalog order and first-reference assignment order are intentionally
different identities. A downstream phase cannot renumber markers from layout,
derive reservation from coordinates, merge carry into the body cursor, or use
trace/manifest JSON as a receipt.

## Public M3 advanced-pagination ownership

These rows are the ownership adopted by
[ADR-0031](../adr/ADR-0031-advanced-pagination-profiles.md), implemented in
private slices by MI3-09 through MI3-11, and published together by MI3-12.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| closed `header-footer-1`, `columns-1`, and `float-1` domains and their immutable profile-receipt fingerprints | `typaxis-machine-profile` advanced descriptor/preflight owner | derive the target capability entries, typed contract-1.3 preflight, rejection matrix, and one combined fixture per profile |
| independent frozen 1.3 DTO/decoder/encoder and complete versioned Schema registry | `typaxis-document-package` version-dispatch owner plus Schema integration owner | keep frozen 1.0/1.1/1.2 registries isolated; populate every required neutral field by typed conversion and retain the versioned 1.3 aliases beside current 1.4 |
| horizontal/LTR master shape, checked trim/body/margin relation, first/left/right selection, and PDF page boxes | page-master geometry/selection owner | issue selected MasterId and MediaBox/CropBox/TrimBox receipts; Display/PDF cannot infer boxes from paint bounds |
| page-region NodeIds/content closure and dense header/footer repetition | syntax index then canonical flow-registry and pagination owners | allocate one MasterId-bound source FlowId per present region, re-evaluate it independently on selected pages, and reject split/carry or body-cursor substitution |
| checked column count/gap partition, last-column residual, and Column FlowIds | column-frame registry owner | issue exact left-to-right frame geometry and source-body before/after cursors without caller or worker ordering |
| `typaxis.column-balance-candidates/1` input, strictly increasing target sequence, candidate permits, and selected final target | pagination balance-budget owner | allow candidate exactly at `max_column_balance_candidates`; emit `G6003` on candidate oscillation or before max+1 and expose only the selected candidate |
| Figure anchor consumption, FIFO queue identity, finite here/top/bottom decisions, and next-page carry | float queue owner under pagination | issue `typaxis.float-queue/1`; keep Float FlowId/caption progress separate from body and emit `G6004` before queue/carry max+1 |
| selected master/boxes, canonical header/body-column/footer frames, repetitions, balance, float placements/carries, and terminal closure | advanced selected-layout owner | issue `typaxis.advanced-pagination-selected-layout/1`; reject missing/extra/reordered/same-position facts as `I9190` |
| exact frame/float Display commands and reopened PDF page-box/command observations | Display owner then PDF graph/serializer owners | issue `typaxis.advanced-pagination-paint-closure/1` bound to the selected layout rather than coordinate-sorted or caller-authored observations |
| byte-identical conditional `advanced_pagination` trace/manifest projection | manifest owned-facts owner | require the member for built advanced profiles, forbid it for old profiles, and serialize no discarded candidate or uncommitted queue state |
| current-contract switch, public profile registration, private-runner removal, and `m3-all.json` publication | MI3-12 integration gate | atomically exposed 1.3 after independent Schema, exact-limit, tamper, PDF, reproducibility, and documented-host closure |

The public progress suffix is `ResourcesAdmitted ->
AdvancedFlowRegistryValidated -> AdvancedPaginationSelected -> DisplayClosed ->
PdfGraphFrozen`. A column frame is a view over the body source cursor; a page
region and Float FlowId have their own terminals. Parent edges express typed
ownership and nesting, never permission to flatten those cursors into the body
continuation.

## Adopted M4 semantic-container and declared-media ownership

These target rows are adopted by
[ADR-0032](../adr/ADR-0032-semantic-container-and-declared-media.md). They had
no implemented Rust type, Schema file, or CLI surface at MI4-01; MI4-02 and
later slices implemented them behind the non-current 1.4 staging boundary,
and MI4-13 moved the complete set into public ownership.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| current contract 1.4, frozen independent 1.0-1.3 Schema registries, and atomic publication/version dispatch | document-package and Schema integration owners under the completed MI4-13 gate | keep frozen 1.0 through 1.3 isolated; no partial decoder, alias, artifact, or profile exposure |
| untrusted semantic-container Wire DTO and closed `SemanticContainerKind` | `typaxis-document-package` | require the exact block-only record and reject unknown/missing/extra members before domain lowering |
| untrusted `ImageMediaType` / `FontMediaType` wire enums and version-exact omission/encoding | `typaxis-document-package` | decode required 1.4 declarations; old encoders omit only provenance-bound legacy absence and never drop declared values |
| trusted `LegacyUnspecified` / `Declared(typed media)` declarations and semantic-container node | `typaxis-document`, issued only by sealed `typaxis-syntax` lowering | bind raw contract provenance, NodeId/SourceSpan/child ownership, and prevent callers from attaching legacy absence to 1.4 |
| production-profile admission of the three typed container kinds, semantic nonempty/nesting policy, allowed style-property set, and allowed media set | `typaxis-machine-profile` descriptor/preflight owner | issue a profile/package/style/declaration/session-bound policy receipt before resource open or flow allocation |
| `semantic_container` selector parsing, cascade/inheritance/applicability, and `SemanticContainerComputedStyle` | `typaxis-style` | retain typed kind/style in the layout receipt so Display/PDF never compare the raw wire kind |
| one canonical container FlowId, parent edge, terminal, fragment sequence, and typed grouping/structure binding | flow-registry, pagination, and selected-state owners | preserve the wrapper across page splits; ordinary child items stay in the container flow while existing/nested subflow owners retain independent FlowIds |
| stable resource bytes plus decoder-issued actual image/font container and outline kind | `typaxis-resource-admission` | exact-match the profile-permitted declaration before expensive media decode/font outline work; URI suffix and caller strings have no authority |
| reference-source 1.4 `media_type` population | shared `dump-ast` exporter consuming same-session resource-admission attestation | emit a declared value only after stable attestation; failure writes no partial JSON and never emits legacy absence |
| `/Result`, `/Proof`, `/Exercise` to `/Div` role mapping and one structure owner across all selected fragments | ADR-0035 structure-registry owner; MI4-09 implements it | retain canonical child reading order and prevent outline/tag reconstruction from paint or PDF object order |
| declaration/attestation resource projection in the M4 manifest branch | manifest owned-facts owner | built M4 records require declared/non-null exact match; pre-resource legacy failure alone may carry legacy/null; frozen old images retain their existing PNG attestation and old fonts gain no new field |

Target progress extends the public chain after the completed publication:

```text
PackageValidated
  -> M4CapabilityValidated
  -> DeclaredMediaPolicyValidated
  -> MediaAttested
  -> SemanticContainerFlowRegistryValidated
  -> LayoutSelected
  -> StructureBound
```

Candidate registration remains before capability preflight, but is not a
resource open or attestation. A downstream owner cannot infer a semantic kind
from classes/text, infer media from a path/PDF object, synthesize a declaration
from legacy input, or flatten the container continuation into its parent.

## Adopted M4 math, safe-vector, and alternative ownership

These target rows are adopted by
[ADR-0033](../adr/ADR-0033-math-safe-vector-and-alternative-binding.md).
MI4-04 and MI4-05 implemented them in the independent non-current 1.4
staging registry; MI4-13 published them with the complete production profile.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| closed `inline_math` / `display_math` Wire DTO, required `typaxis-math` version `1` source binding, and producer `speech` member | contract-1.4 `typaxis-document-package` decoder/encoder | reject missing/extra/wrong-typed members and another source version as versioned input; never infer kind from delimiters or text |
| contract-1.4 `M4ResourceLimits`, hard maxima, and base-plus-extension effective-limit fingerprint | contract-1.4 config decoder plus the sealed core limit validator | issue session/package-bound vector and math permits; omit the extension from frozen old aliases/default JCS and prevent resource/retry resets or foreign-limit substitution |
| NodeId/SourceSpan/TextSpan identity, exact source bytes, and producer alternative | sealed `typaxis-syntax` lowering | issue a package-bound math-source input to the math parser during syntax validation; never generate speech or visual fallback |
| `typaxis.math-parser/1`, `typaxis.math-formatter/1`, typed AST/fingerprint, fixed grammar, and math-computation receipt | in-tree `typaxis-math` | parse exact bytes before capability/resource work, prove canonical formatter round-trip, and later produce bounded font-metric-driven dimensions/glyph/rule/path output without package/PDF authority |
| closed math node/version/placement support and old-profile rejection | machine-profile preflight consuming the validated package | reject page-region or unsupported profile use before resource open and bind the accepted math set into the target profile receipt |
| admitted math font/MATH-table/glyph input and final `typaxis.math-binding/1` receipt | layout binding owner consuming the package/profile/limits/LayoutEpoch/resource chain and opaque math computation | bind source, span, kind, speech, font hash, dimensions, baseline, work, and vector fingerprint into one `MathReceiptKey` |
| one atomic inline item or one independent display `MathFlowId` and terminal | inline itemizer or `typaxis.math-flow/1` registry owner | forbid breaks inside an expression; parent display flow advances only after the exact math terminal |
| selected parent/math FlowIds, page/frame/fragment/paint ordinals, origin, and transform extension of the MathReceiptKey | selected-state owner | preserve one atomic math owner and reject wrong flow/page/fragment/font/vector/alternative substitution as `I9190` |
| exact producer speech to PDF `/ActualText`, canonical glyph/rule/path commands, and observed serialized paint | Display owner then PDF graph/serializer owner | extract the same scalar sequence while retaining visual vector output; source text, glyph names, or generated speech cannot replace the alternative |
| one `/Formula` structure owner and `/Alt` equal to the same producer speech | ADR-0035 structure-registry owner, implemented by MI4-09 while consuming the MathReceiptKey | add tree/MCID/language closure without changing, splitting, or regenerating the adopted alternative |
| declared `ImageMediaType::SvgSafe1` wire value `svg-safe-1` and profile admission | document-package/domain lowering then machine-profile declared-media policy | reject missing/legacy/unknown/disallowed media before resource open; URI suffix and host MIME have no authority |
| stable-byte `typaxis.safe-svg-parser/1`, fixed element/paint/geometry subset, inclusive vector permits, canonical `typaxis.safe-vector-ir/1`, and `AdmittedImageMediaKind::SafeVector` | `typaxis-resource-admission` | issue intrinsic size/view box/allocation charge/IR fingerprint attestation after bounded validation and before layout/PDF; perform no filesystem, network, font, CSS, script, or browser work |
| logical SafeVector use, selected Figure placement, and final PDF-ready Form plan | layout/selected-state, Display usage, then `typaxis-resources` finalization owners | bind the existing ImageResourceId and admitted bytes/IR fingerprint to one DrawVector use and canonical Form plan |
| dense PDF resource/object names and actual path/clip/fill/stroke Form XObject | PDF backend alone | serialize the frozen vector plan without reparsing SVG or raster fallback and reopen the actual object observation |
| math/source/alternative/vector and SafeVector declaration/attestation/IR/usage/object facts | M4 manifest owned-facts owner | require bidirectional receipt closure and omit all fields from frozen old-profile artifact branches |
| parser/formatter/IR dependency and tool identities | in-tree implementations plus `typaxis-testkit` dependency audit | permit only ADR-0033's workspace edges, forbid external math/XML/SVG/CSS/browser/speech/network dependencies, and fingerprint semantic algorithm identities |

Target progress extends the private M4 chain as follows; publication exposes
only the complete combined chain:

```text
PackageValidated (including MathSourceValidated,
  DocumentMetadataValidated,
  ComputedLanguageRegistryValidated,
  OutlineRegistryValidated)
  -> M4CapabilityValidated
  -> DeclaredMediaPolicyValidated
  -> MediaAttested (including SafeVectorAttested,
       JpegDecodedAndSanitized, SfntCff1AttestedAndPermitted)
  -> SemanticContainerFlowRegistryValidated
  -> MathFontAndLayoutBound
  -> MathFlowRegistryValidated
  -> MathAndVectorLayoutSelected
  -> BookNavigationSelected
  -> TaggedStructureBound (including FormulaStructureBound)
  -> DisplayClosed
  -> FontGlyphClosureBound
  -> ResourcePlansFrozen
  -> PdfGraphFrozen
  -> PdfBytesVerified
  -> BookNavigationPdfObserved
  -> TaggedPdfObserved
  -> AccessibilityValidated
```

MI4-04/05/07 slice-local runners may produce non-public vector/math/navigation
Display and PDF evidence for their own tests, but those receipts do not issue
`TaggedStructureBound`, cannot be promoted to this combined progress type, and
cannot reach publication. Once the tagged owner exists, the combined target
uses the order above without a bypass edge.

ADR-0033 native Safe SVG and native math remain separate typed paths: a native
math node never acquires an ImageResourceId, while a version-1 SafeVector image
never acquires a math source or alternative. ADR-0037 adds the separately
versioned producer-composed path below; no path may reconstruct authority from
trace, manifest, coordinates, PDF objects, a URI suffix, or caller-authored
hashes.

## Adopted producer-composed math-vector ownership

These target rows are adopted by
[ADR-0037](../adr/ADR-0037-producer-composed-math-vector.md). MI4-V01 fixed the
producer-interface corpus, MI4-V03 through MI4-V18 implemented private 1.4
slices, MI4-V19 closed feature-local external evidence, and MI4-13 published
the complete profile. Existing ADR-0033 through ADR-0036 `/1` owners remain
frozen.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| four closed Wire kinds, producer metrics/spacing/source/alternative, `svg-safe-2` required hash/provenance, and kind-conditional members | contract-1.4 `typaxis-document-package` decoder/encoder | reject missing/extra/wrong-typed fields and preserve exact canonical integers/UTF-8; never infer kind, TeX, alternative, or media |
| trusted typed kinds, metric/source/alternative records, and content-owning semantic nodes | sealed `typaxis-syntax` lowering into `typaxis-document` | issue package/session/source-bound records only after dense NodeId/TextMap/meaningful-text/metric validation; caller raw parts cannot issue receipts |
| `typaxis.precomposed-vector-metrics/1` relation, one uniform intrinsic-to-viewport scale, and source/vector/alternative binding | syntax/resource-aware layout binding owner consuming stable SafeVector attestation | bind advance/ascent/descent/origin/baseline/viewport, exact TeX/Alt/ActualText, resource/provenance/parser/IR, limits, and LayoutEpoch without parsing TeX |
| precomposed-vector selector/cascade and exact property applicability | `typaxis-style` under `typaxis.precomposed-vector-style/1` | issue computed block receipts for `math_vector_block`/`vector_figure`; frozen basic style `/1` cannot authorize either kind |
| accepted kind-to-media matrix, language-owner set, resource set `/2`, and public capability projection | `typaxis-machine-profile` descriptor/preflight owner | reject disallowed kinds/media/style before resource open and bind package/session/declaration/limits; publish the exact eight-profile descriptor fixed by MI4-13 |
| `svg-safe-2`, parser/IR/allocation `/2`, exact currentColor/paint-alpha extension, and stable-byte hash/provenance attestation | `typaxis-resource-admission` | accept only ADR-0033 Safe-SVG 1 plus the closed delta, issue deterministic intrinsic geometry/IR/alpha facts, and perform no TeX/XML-browser/network/font work |
| atomic inline AL/isolate item, vector-boundary spacing, dynamic line ascent/descent, and visual-frame fit | itemizer/line-break/layout owners under `typaxis.atomic-vector-inline/1` | use advance for line width, producer baseline for paint, existing Unicode/Japanese break permission, and terminal `L5100` rather than internal split/fallback |
| dense nominal `MathVectorFlowId`, equation-number source leaf/shape, atomic block alignment and pagination | `typaxis.math-vector-flow/1` and block-layout owners | keep native MathFlowId `/1` separate, place number independently, preserve one terminal, and reject overlap/empty-frame oversize without shrink/crop/split |
| selected pen/baseline/viewport/matrix and one occurrence charge | selected-state owner under `typaxis.precomposed-vector-layout/1` | bind source-order flow/line/block to page/frame/paint without URI/SVG/PDF authority |
| logical DrawVector `/2`, typed `VectorContentKey`, canonical alias/dedupe plan, and Form/ExtGState plan set `/2` | Display then `typaxis-resources` finalization owners | order by verified content-key tuple and alpha pair, share one Form across aliases/colors, retain zero-use facts, and leave absolute object allocation to the final graph |
| path/clip/fill/stroke/alpha Form plus page-local resolved-color `Do` usage and actual object observation | PDF graph/serializer owners | serialize only sealed plans, apply one viewport/page transform, keep MCID/Alt/ActualText/Lang out of reusable Forms, and never rasterize or reparse SVG |
| computed-language/book-navigation `/2` owner set and Formula/Figure/number structure/marked-content/tagged-PDF `/2` mapping | syntax navigation, layout-contract structure, selected Display, PDF, validator, and assessment owners at their existing phase boundaries | add the four kinds without changing `/1`; outer MCR owns MCID and optional inner property Span encloses only `Do` |
| SafeVector `/2`, math-vector `/1`, book-navigation `/2`, and tagged-PDF `/2` manifest facts | versioned production manifest owned-facts owners | close resource aliases/counts, metric/source/alternative, flow/placement, PDF, language, and structure bidirectionally in acyclic dependency order |
| milestone status/dependency and final publication | docs/25 master plan and MI4-13 atomic publication owner | require `MI4-V18 + MI4-11 + MI4-12 -> MI4-V19 -> MI4-13`; docs/27 alone owns detailed V tasks/acceptance |

The producer-composed progress chain extends the combined M4 chain:

```text
PackageValidated
  -> ProducerVectorSyntaxValidated
  -> M4CapabilityValidated
  -> SafeSvg2Attested
  -> PrecomposedMathVectorBound
  -> AtomicVectorLayoutSelected
  -> DrawVectorDisplay2Closed
  -> VectorFormPlans2Frozen
  -> PdfGraphFrozen
  -> BookNavigation2Observed
  -> TaggedPdf2Observed
  -> ProducerVectorEvidenceValidated
```

The full profile additionally requires the existing semantic-container,
native-math, PNG/JPEG/TrueType/CFF, metadata/navigation, and tagged-PDF chains.
No producer-vector receipt substitutes for those dependencies. V19 success
was necessary but not sufficient; MI4-13 performed the atomic alias and
capability publication.

## Adopted M4 JPEG and OpenType/CFF resource ownership

These target rows are adopted by
[ADR-0036](../adr/ADR-0036-jpeg-and-opentype-cff-resource-profiles.md).
MI4-11 and MI4-12 implemented the two components separately in independent
non-current 1.4 staging. ADR-0036's `/1` set stays frozen; after MI4-V19,
MI4-13 advertised the components inside ADR-0037's complete `/2` resource set
and moved it into public ownership.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| exact `jpeg-baseline` and `sfnt-cff1` Wire values and versioned required-member encoding | contract-1.4 `typaxis-document-package` decoder/encoder | reject alias/missing/null/unknown values as `P1102`; preserve the existing PNG/SafeVector/TrueType values and frozen encoders |
| trusted `ImageMediaType::JpegBaseline` / `FontMediaType::SfntCff1` declaration plus raw-contract provenance | sealed `typaxis-syntax` lowering into `typaxis-document` | issue declared values only for 1.4 and prevent suffix/MIME/caller text from acquiring typed authority |
| five exact ADR-0036 resource-component meanings, frozen ordered `typaxis.production-book-resource-set/1`, and ADR-0037 complete `typaxis.production-book-resource-set/2` replacing only SafeVector with `/2` and adding `svg-safe-2` media | `typaxis-machine-profile` descriptor/preflight owner | issue a package/session/declaration/effective-limits policy receipt before resource open; no generic image/font flag or partial component set, and publication uses only the complete `/2` set |
| stable JPEG marker/frame/scan/table/entropy facts and `AdmittedImageMediaKind::JpegBaseline` | iterative in-tree `typaxis-resource-admission` marker preflight | exact-match the declaration and acquire bytes/pixels/decoded/scratch permits before constructing the external decoder |
| exact Gray8/RGB8 decoded byte count/hash under `jpeg-decoder = 0.3.2` platform-independent code | `typaxis-resource-admission` decoder wrapper consuming the marker receipt | issue validity evidence only; it cannot choose media, metadata, geometry, sanitizer bytes, or PDF policy |
| exact JFIF APP0 removal, otherwise byte-preserved normalized stream length/hash | in-tree `typaxis.jpeg-segment-sanitizer/1` under `typaxis-resource-admission` | issue one deterministic sanitized-stream receipt after full decode and before any DCTDecode plan; never re-encode pixels |
| standalone `OTTO` directory/table/CFF1 cross-check and `AdmittedFontMediaKind::SfntCff1` | `typaxis-font` bounded sfnt/CFF admission owner, invoked only by `typaxis-resource-admission` over stable bytes | attest only face-index-zero name-keyed CFF1 with the closed table/operator domain; `read-fonts` supplies typed views but no policy receipt |
| exact `OS/2.fsType` CFF1 embedding decision | `typaxis.cff1-embedding-permission/1` under the same font admission owner | accept only 0/0x0004/0x0008 and seal the raw value; reject restricted/no-subset/bitmap/reserved state before shaping, subset, or PDF without revising the existing TrueType component |
| contract-1.4 font-table/glyph/subroutine/operation/outline/subset limits and base-plus-extension fingerprint | contract-1.4 config decoder plus sealed core M4-limit validator/budget owners | apply inclusive max/max+1 rules exactly once in dense FontFaceId/source-GID order; keep frozen old config/descriptors unchanged |
| per-FontInstanceId selected source-GID union, `.notdef`, ascending dense CID/GID map, and same-face/epoch closure | `typaxis.cff1-glyph-closure/1` in `typaxis-font`, consuming sealed shaping/generated/math/Display usage | issue one immutable CFF1 instance glyph-closure receipt; no PDF/manifest/cmap scan may invent or remove a selected glyph |
| one bounded Type 2 outline observation per distinct selected face/source-GID and one canonical hint/subroutine-stripped CID-keyed CFF1 subset bytes/hash/name per instance | `typaxis.cff1-charstring-evaluator/1` and `typaxis.cff1-subset/1` in `typaxis-font` | consume permission, instance glyph closure, limits, and stable face; share only sealed same-face outlines and write fixed table/CID order with no platform writer, full-font, or raster fallback |
| JPEG DCTDecode and CFF FontFile3/OpenType/CIDFontType0 frozen resource plans | `typaxis-resources` late finalizer consuming selected Display usage and the sealed admission/subset receipts | issue backend-name/object-free plans; unused admitted resources get no plan, and TrueType/JPEG/PNG/CFF plan substitution is impossible |
| image XObject and Type0/CIDFont/descriptor/program/ToUnicode/CIDSet objects | PDF graph/serializer owners alone | assign canonical resource names/object IDs, serialize only the frozen plan, and issue exact dictionary/stream observations over final PDF bytes |
| declaration/attestation/source/decoded-or-glyph/transform-or-subset/permission/plan/object facts | versioned M4 manifest owned-facts owner | require bidirectional same-ImageResourceId or same-FontFaceId/FontInstanceId closure; JSON cannot create any upstream fact |
| exact versions/checksums/licenses/features/MSRV/direct and forbidden dependency edges | `typaxis-testkit` locked dependency/supply-chain audit | fail unexpected feature/package/native edge or advisory evidence; an upgrade or replacement requires an identity/ADR review |

JPEG and CFF share neither a parser nor a generic embedding plan. The JPEG
path closes one ImageResourceId from source hash through decoded observation,
sanitized hash, selected DrawImage, and DCTDecode object. The CFF path closes
one FontFaceId/FontInstanceId from source/face/license through selected glyphs,
subset hash, FontFile3 plan, and all composite-font objects. Equal source bytes
or hashes do not permit receipt reuse across logical IDs.

The source exporter consumes the same successful admission/permission receipt
as package validation; failure writes no partial JSON. A malformed,
unsupported, declaration-mismatched, restricted, max+1, or closure-invalid
resource is terminal with no alternate decoder, PNG/TrueType/full-font/raster
fallback, partial plan, or manifest repair. These owners occupy the
`MediaAttested`, `FontGlyphClosureBound`, and `ResourcePlansFrozen` stages in
the combined M4 progress chain above.

## Adopted M4 metadata, language, and outline ownership

These target rows are adopted by
[ADR-0034](../adr/ADR-0034-document-metadata-language-and-outline.md).
MI4-07 implemented them in the independent non-current 1.4 staging registry;
MI4-13 published them with the complete production profile.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| required closed `metadata`, `document.language`, optional node `language`, required `outline.entries`, and nullable semantic-container `anchor_id` Wire members | contract-1.4 `typaxis-document-package` decoder/encoder | preserve exact producer bytes and reject missing/extra/wrong-typed members; never infer values from source, host, or another field |
| nonempty/control-free metadata strings, canonical keyword order, exact `typaxis.utc-second/1` dates, and modified/created relation | sealed `typaxis-syntax` metadata validator | issue `DocumentMetadataReceipt` before profile/layout/PDF work; clocks and file times have no document-fact authority |
| registry-independent `typaxis.bcp47-language/1` parse/canonicalization and logical-owner inheritance | sealed `typaxis-syntax` language owner | issue `ComputedLanguageRegistryReceipt` for every language-capable NodeId; shaping, style, layout order, and host locale cannot reinterpret it |
| dense source-preorder outline IDs, exact level/parent stack, heading/container kind and source binding, unique AnchorId, and anchor-owner equality | sealed `typaxis-syntax` outline owner | issue `ValidatedOutlineRegistryReceipt`; never infer labels, entries, hierarchy, or coordinates from headings, paint, or PDF |
| closed metadata/language/outline feature set and old-profile rejection | machine-profile preflight consuming the three validated receipts | issue the production profile authorization before resource open/layout/PDF without broadening any old descriptor |
| one-time AST/text/depth/fragment/object charges using the existing inclusive limits | syntax, selected-layout, and PDF budget owners for their own units | refuse max+1 before receipt, selected record, allocation, or serialization; retry and foreign receipts cannot reset aggregates |
| exact semantic-container/heading anchor point and existing selected named-destination registry entry | selected-layout destination owner | bind source owner, selected page/frame/view/point, LayoutEpoch, and destination-registry fingerprint without caller page/coordinate fallback |
| metadata/language/outline plus selected destination extension | book-navigation selected-state owner | issue `BookNavigationSelectedReceipt` and prove every validated entry has exactly one selected target before object allocation |
| Info dictionary, navigation-only `typaxis.book-xmp/1` Metadata stream, catalog `/Lang`, marked-content `/Lang`, and canonical outline object graph | PDF graph/serializer owners alone | allocate deterministic roles, reference the existing name-tree key, and issue `BookNavigationPdfObservation`; ADR-0035 versions the tagged projection as `typaxis.book-xmp/2` rather than changing version 1 |
| structure-element language and optional outline-item `/SE` source relation | ADR-0035 tagged-structure owner, implemented by MI4-09 while consuming the computed-language/outline receipts | add exact structure bindings without changing canonical tags, source owners, labels, hierarchy, or destinations |
| metadata/language/navigation manifest facts and independent decoded PDF observations | manifest owned-facts owner and external validator, respectively | require bidirectional closure over the prior receipts; neither JSON nor the validator can manufacture upstream authority |

The metadata, language, and outline owners feed the combined M4
progress chain above. An interned language value does not erase its logical
per-NodeId aggregate charge; a selected destination does not authorize a new
outline entry; and Info/XMP/catalog/outline bytes cannot repair invalid Wire or
substitute for an owner-issued receipt.

## Adopted M4 tagged-PDF and accessibility-validation ownership

These target rows are adopted by
[ADR-0035](../adr/ADR-0035-tagged-pdf-structure-and-validation.md). MI4-09
implemented them in independent non-current 1.4 staging; MI4-13 published the
complete profile and its pinned PDF/UA-1 evidence contract.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| exhaustive source role vocabulary, generated wrapper slots, dense StructureNodeId allocation, source parentage, and logical reading order | `typaxis-layout-contract` structure-registry builder consuming sealed syntax receipts plus the profile-bound dependency-inversion authorization | issue a PDF-independent `StructureRole` registry before layout; coordinates, typography, paint order, and PDF objects have no role or order authority |
| closed PDF/UA-1 production subset, including title, heading sequence, non-whitespace semantic Figure/TH/P/heading content, catalog-language Link/outline strings, headed Table, name-contributing Link content, footnote placement, and a classification rule for every painting variant | machine-profile accessibility preflight | issue the sealed preflight receipt and syntax-owned lower authorization; reject unsupported semantics as `L5100` before layout/PDF without broadening an old profile, while actual selected paint/annotation closure remains downstream |
| selected fragments, repetitions, generated labels, and decoration classified as one structure owner or one artifact occurrence | selected-layout owner consuming structure, flow, resource, math, language, and navigation receipts | issue `SelectedStructureBindingReceipt`; require every selected occurrence exactly once and retain both semantic and physical ordinals |
| DisplayPaintId binding, maximal marked-content groups, and dense page-local MCIDs in final paint order | Display binding owner, then the `typaxis-display-list` PDF-profile finalizer separate from the Display value and serializer | consume the layout-contract receipt through the existing `typaxis-layout` re-export, keep Display free of MCID/PDF names/objects, issue `MarkedContentPlanReceipt`, and let `typaxis-pdf` consume it only through its existing display-list edge |
| StructTreeRoot, RoleMap, StructElem/MCR/OBJR objects, ParentTree, IDTree, page `/StructParents`, annotation `/StructParent`, and `typaxis.book-xmp/2` | PDF graph and serializer consuming the closed plans | allocate deterministic later object roles, serialize only receipt-authorized dictionaries, and issue `TaggedPdfObservation` over the exact PDF hash |
| Figure `/Alt`, Formula `/Alt` and `/ActualText`, structure/marked-content `/Lang`, Link `/Contents`, TH IDs/TD Headers, Note/reference relation, and outline `/SE` | structure registry plus the earlier alternative/language/grid/footnote/link/outline owners | reuse the exact upstream facts; missing, extra, duplicate, reordered, or wrong-owner closure is `I9190`, never a repair opportunity |
| generated structure-node, depth, MCR/artifact, text, MCID, PDF-object, output, and spool charges | existing syntax, selected-layout, text, PDF, and output budget owners | apply one-time inclusive maxima before allocation/serialization; add no synonymous accessibility limit or retry reset |
| writer-independent PDF observation, exact veraPDF Greenfield 1.30.2 `ua1` report, empty warning allowlist, and complete version-2 (`/2`) Matterhorn Protocol 1.02 assessment ledger | in-tree validator and release-evidence aggregator, never the writer | require all evidence for the same PDF hash while leaving the frozen version-1 (`/1`) evidence contract unchanged; machine success alone cannot issue a full conformance, accessibility, or legal claim |

The tagged owners extend the combined M4 progress chain above. MI4-09 supplied
the structure and validation receipts and local `TaggedPdfObserved` and
`AccessibilityValidated` observations; MI4-V19 and MI4-13 closed external
evidence and public release support respectively.

## Shared base ownership (originating in contract 1.0)

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| host entry/project/config/resource-root paths and explicit inspection order | HostAdmissionContext builder | admission only; never serialize host paths or use order as first-match precedence |
| host output/sidecar paths, initial pairwise target-alias rejection, file/stdout sink, and replace policy | BuildExecutionContext builder | configure writes; serialize only OutputSink, never HostPath |
| one-shot output session, current target-identity rechecks, and platform atomic PDF/manifest writes | non-cloneable BuildOutputCommitContext | exist for every build; consume only serializer receipt when manifest is omitted or same-session sealed terminal preflight when requested |
| configured root resolution, root-alias rejection, and session capability | host admission / admitted-root owner | issue one opaque `AdmittedRootSet`; raw HostPath arrays are never a resolver trust input |
| contained candidate lookup, multi-root ambiguity rejection, no-follow opened-handle identity, and immutable extent | source/resource admission opener | issue a same-root-set `VerifiedResourceSource` or fail closed |
| source path admission, decoding, include graph/depth, dense SourceId discovery order, and trusted package issuance | sealed Parser / IncludeResolver in syntax | issue `ValidatedIncludeGraph`, immutable `SourceCatalog`, and `ValidatedParsedPackage`; flat packages and fixture features cannot recreate the receipt |
| URI parsing, scheme/control/whitespace/length validation | syntax | read `SafeUri` only |
| font/image path admission, bounded bytes/hash, bytes-derived metadata, and dense declaration closure | `typaxis-resource-admission` resolver | issue an immutable admitted-resource ledger; shaping/layout never accept caller-assembled admitted records |
| normalization and source mapping | text | query owning-buffer-local ranges |
| initial and state-specific generated text keys/bytes and canonical GeneratedTextBufferId allocation | LayoutPassCoordinator / GeneratedTextStore | derive the initial seed from validated package/limits without a caller store; sort GeneratedBufferKey before allocation; retain each materialized overlay through Display construction |
| semantic node/anchor/generated-site identity and dense typed preorder | document index builder | retain logical IDs and issue the complete generated-site registry, including one list-marker site per ListItem |
| canonical list-marker bytes | validated package generated-text owner | derive checked ordered numbering or U+2022 from the AST and reject any different overlay bytes |
| closed property validation, style precedence, and typed computed values | style | provide ResolvedTextStyle and requested PageName |
| declared family alias to admitted FontFaceId binding | admitted resource resolver | expose byte-exact canonical family table to StyleResolver |
| LayoutEpoch plus package/style/ledger/instance-bound shape font selection | `typaxis-layout-contract` | issue one sealed `ShapeFontSelectionReceipt` for the computed face and exact epoch; layout re-exports it and shaping cannot accept raw font identity inputs |
| parsed/generated shaping text provenance, site owner, and canonical style owner | syntax package text-receipt issuer | derive ownership from typed AST/site registry; use the first text-producing FootnoteDefinition descendant by typed preorder and never accept a caller-selected owner |
| page index and derived PageSelectionContext | pagination | pass pre-master context to selector |
| page-master rule winner and selected PageContext | PageMasterSelector | pass exactly one known master/frame set to Fragmenter |
| logical adjacency, bidi/script itemization, Profile 1.0 language/features, glyph selection, and positioning | crate-owned shaping itemizer/backend | privately issue `ShapeRequest` from adjacent package text receipts with canonical `language=None`/empty features; consume a `ShapeFontSelectionReceipt` from the lower contract crate |
| linked shaper backend/version selection fact | closed ShaperIdentity registry | project the registered selection into manifest facts; do not treat the value alone as proof that an implementation shaped every run |
| paragraph items and legal breaks | linebreak | score/select explicit items |
| canonical FlowTree positions and terminal boundary | flow builder from validated layout IR | issue structured positions; use explicit root End for nonblank flow and sole DocumentStart for blank flow; require More to advance to a nonterminal member and Exhausted to name the exact terminal; callers cannot provide boundary order or opaque progress keys |
| block fragmentation and anchor discovery | layout Fragmenter + pagination permit | bind each package-registered anchor ID/owner to the exact selected page/frame/column/local point under a complete layout-budget token |
| state-dependent reflow orchestration and resolved reference transition | LayoutPassCoordinator / sealed ReferenceTransitionReceipt owner | retain each state's exact paint overlay, bind next working overlay to previous pages/anchors/package sites and the same nonserialized PaginationSessionId before work, and rebuild text/linebreak/flow per pass; reference workspace issues only the zero-site unchanged transition |
| page break, materialized state, fallback score, and selected-state receipt | pagination | emit page plan without shaping; selected receipt binds Display construction |
| canonical PaginationFingerprintRecord and state fingerprint bytes | PaginationFingerprintEncoder under LayoutPassCoordinator | include canonical placed-anchor facts, reject duplicate/noncanonical state components, then JCS/SHA-256 |
| page paint, destinations, annotations, and selected-layout identity | display-list builder | derive destinations exactly from selected placed anchors and issue selected-bound validated Display; bare wire validation is not a trusted phase token |
| parsed/generated span remap to dense DisplayTextBufferId | display-list builder | consume a package/selected-bound DisplayTextMap and expose only DisplayDocument's canonical text-buffer table plus DisplayTextSpan |
| logical resource usage union and duplicate-use elimination | resource collector | pass one usage record per logical ID to finalization |
| PDF-profile subset, CID and type-specific CID/GID mapping, extraction, image encoding, descriptor metrics, and indirect-object blueprint | late resource finalizer / verified encoder receipt | bind the selected epoch ledger and issue backend-identity-free `FrozenPdfResourcePlans`; require CIDToGIDMap only for a profile such as current CIDFontType2, and prove direct CFF charset mapping where its plan omits that object; caller-supplied encoded bytes are untrusted |
| embedded subset PostScript name/tag | deterministic subsetter + late resource finalizer | rewrite the font `name` table, re-extract and bind the exact name in a sealed receipt, then verify the FontInstanceId-derived value |
| PDF resource names, destination/annotation materialization, and object IDs | PDF backend canonical allocator | preflight all typed object roles, consume selected-bound Display/frozen plans, reuse the verified subset PostScript name, then allocate dense IDs/resource names internally |
| stream Filter/DecodeParms/Length dictionary materialization | PDF serializer | derive from frozen encoding policy and encoded bytes |
| defaults/file/environment/CLI resolution and canonical set-array normalization | config loader/CLI | pass immutable effective config |
| optional post-config manifest target/config eligibility, publication session, and resolved-config JCS hash | non-cloneable ManifestPublicationContext | exist only when requested; issue same-output-session admission/preflight capabilities without inventing missing config facts |
| source/resource/layout/PDF facts, terminal manifest record, and canonical manifest bytes | manifest owned-facts factory | project only from validated package/admission/pagination/serializer artifacts; never accept caller-authored trusted records or expose a trusted manifest before atomic publication |
| PDF-then-manifest terminal publication sequence and actual sink receipts | self-consuming BuildOutputCommitContext terminal committer | publish each requested file individually in fixed order; never claim a multi-file transaction, retain the already-visible PDF receipt in a later manifest pre-publication error, and retain the complete publication in any post-publish directory-sync error |

A downstream phase must not reconstruct an upstream decision from presentation data. In particular, PDF must not infer paragraphs from coordinates, pagination must not shape text, late finalization must not reopen an arbitrary filesystem path, and no phase may unwrap an error/fatal result as a success value.

The Display List boundary is PDF-independent. Late resource finalization and every downstream phase are profile 1.0 PDF-specific; only the PDF backend may introduce backend handles, PDF resource names, and object IDs.
