# Phase ownership

## Implemented public machine-input ownership

The following rows are the ownership adopted by [ADR-0027](../adr/ADR-0027-machine-document-package-ingestion.md) and extended by [ADR-0028](../adr/ADR-0028-basic-document-profile.md), [ADR-0029](../adr/ADR-0029-table-profile.md), and [ADR-0030](../adr/ADR-0030-footnote-profile.md). Public package commands and local host evidence cover all four immutable profiles. The lower table remains the shared base ownership inventory rather than an alternative machine-input path.

| Data or decision | Sole owner | Downstream use |
|---|---|---|
| compiled contained-package/contained-resource/atomic-publish availability tokens | machine-input, resource-admission, and atomic-publisher owners respectively; composed by `typaxis-machine-profile` | drive `profiles[].available`; contained-open tokens drive PACKAGE-before-read `I9110`, while missing atomic publication fails during context construction; CLI does not duplicate booleans |
| fixed `MAX_RESOURCE_ROOTS` and `MAX_HOST_READ_CANDIDATES` | `typaxis-host-admission` | preflight before root identity/open and candidate open; capability JSON projects the same constants |
| package/resource root handles, contained component walk, same-handle snapshot, bounded stable bytes, and host read/write identity ledger | `typaxis-host-admission` | issue generic session-bound host receipts only; never infer logical IDs or canonical records |
| PACKAGE HostPath/default or explicit package-root resolution and root-relative package URI | `typaxis-machine-input` using host-admission receipts | bind one machine admission session; serialize only `PortablePath`, never absolute root/path |
| strict JSON lexical preflight, caller-constructible `WireDocumentPackage`, decoder-issued `DecodedDocumentPackage`, JSON location index, and package JCS hash | `typaxis-document-package` | portable decode/export only; never issue host or trusted syntax authority |
| raw PACKAGE receipt, decoded binding, exact single companion-source set, read budgets, and monotonic machine-input progress | `typaxis-machine-input` | issue `AdmittedMachinePackage`; reject cross-session raw/decoded/source receipt substitution |
| DTO lowering, actual source/TextMap/AST/style/master/resource validation, entry-only closure, and trusted package issuance | sealed `typaxis-syntax::DocumentPackageParser` | issue `ValidatedMachinePackage { ValidatedParsedPackage, provenance }`; no public DTO promotion path |
| immutable `paragraph-1` / `basic-document-1` / `footnote-1` / `table-1` descriptors, host availability, deterministic preflight order, and capability receipt | `typaxis-machine-profile` | generate canonical capability JSON and require the same profile/package/style/session binding at machine layout entry |
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
| frozen 1.2 registry and current 1.3 registry | Schema/contract integration owner | keep every version independent; current aliases, decoder, help, capabilities, fixtures, and generated artifacts switch only at an atomic publication milestone |

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
| independent 1.3 DTO/decoder/encoder and complete versioned Schema registry | `typaxis-document-package` version-dispatch owner plus Schema integration owner | keep frozen 1.0/1.1/1.2 registries isolated; populate every required neutral field by typed conversion and expose the current 1.3 aliases |
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
| PDF-profile subset, CID/CIDToGIDMap, extraction, image encoding, descriptor metrics, and indirect-object blueprint | late resource finalizer / verified encoder receipt | bind the selected epoch ledger and issue backend-identity-free `FrozenPdfResourcePlans`; caller-supplied encoded bytes are untrusted |
| embedded subset PostScript name/tag | deterministic subsetter + late resource finalizer | rewrite the font `name` table, re-extract and bind the exact name in a sealed receipt, then verify the FontInstanceId-derived value |
| PDF resource names, destination/annotation materialization, and object IDs | PDF backend canonical allocator | preflight all typed object roles, consume selected-bound Display/frozen plans, reuse the verified subset PostScript name, then allocate dense IDs/resource names internally |
| stream Filter/DecodeParms/Length dictionary materialization | PDF serializer | derive from frozen encoding policy and encoded bytes |
| defaults/file/environment/CLI resolution and canonical set-array normalization | config loader/CLI | pass immutable effective config |
| optional post-config manifest target/config eligibility, publication session, and resolved-config JCS hash | non-cloneable ManifestPublicationContext | exist only when requested; issue same-output-session admission/preflight capabilities without inventing missing config facts |
| source/resource/layout/PDF facts, terminal manifest record, and canonical manifest bytes | manifest owned-facts factory | project only from validated package/admission/pagination/serializer artifacts; never accept caller-authored trusted records or expose a trusted manifest before atomic publication |
| PDF-then-manifest terminal publication sequence and actual sink receipts | self-consuming BuildOutputCommitContext terminal committer | publish each requested file individually in fixed order; never claim a multi-file transaction, retain the already-visible PDF receipt in a later manifest pre-publication error, and retain the complete publication in any post-publish directory-sync error |

A downstream phase must not reconstruct an upstream decision from presentation data. In particular, PDF must not infer paragraphs from coordinates, pagination must not shape text, late finalization must not reopen an arbitrary filesystem path, and no phase may unwrap an error/fatal result as a success value.

The Display List boundary is PDF-independent. Late resource finalization and every downstream phase are profile 1.0 PDF-specific; only the PDF backend may introduce backend handles, PDF resource names, and object IDs.
