# Phase ownership

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
| PDF-then-manifest terminal transaction and actual sink receipts | self-consuming BuildOutputCommitContext terminal committer | atomically publish built or failed result; retain the already-visible PDF receipt in a later manifest pre-publication error and retain the complete publication in any post-publish directory-sync error |

A downstream phase must not reconstruct an upstream decision from presentation data. In particular, PDF must not infer paragraphs from coordinates, pagination must not shape text, late finalization must not reopen an arbitrary filesystem path, and no phase may unwrap an error/fatal result as a success value.

The Display List boundary is PDF-independent. Late resource finalization and every downstream phase are profile 1.0 PDF-specific; only the PDF backend may introduce backend handles, PDF resource names, and object IDs.
