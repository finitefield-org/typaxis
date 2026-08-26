# Implementation checklist

このchecklistの`[x]`は、Profile 1.1のcontract invariantまたは明記したdelivery gateにRust type、Schema、validator、public E2Eの対応証拠があることを表す。公開範囲は[producer guide](26-machine-input-cli.md)、milestone completionとrich block/resourceの不足は[docs/25](25-machine-input-pdf-improvements.md)のsupport matrixを正とする。

## Machine input delivery gates

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| reference TSF pipeline | Yes, current 1.1 | Yes, bounded subset | Yes | No |
| DocumentPackage portable Schema/export | Yes, current 1.1 plus frozen 1.0 input | Yes | Yes, package round trip | No |
| sealed machine ingestion | Yes, ADR-0027 | Yes | Yes, Linux fixture gate | No: two-host aggregate pending |
| `typaxis.machine-pdf/paragraph-1` | Yes, closed capability contract | Yes | Yes, Linux combined PDF/sidecars | No: two-host aggregate pending |
| contract 1.1 output | Yes | Yes, current output | Yes | No: two-host aggregate pending |

- [x] ADR-0027 fixes command identity, package-root/resource-root separation, single-source M1, sealed receipt ownership, immutable profile semantics, contract 1.1 migration, and publication order
- [x] `typaxis-machine-input -> typaxis-syntax` is forbidden; syntax remains the sole trusted package issuer
- [x] `paragraph-1` explicitly distinguishes visual heading layout from outline/tagged heading semantics
- [x] target host/document-package/machine-input/machine-profile owners and session-bound receipts are implemented
- [x] public `build-package`, `check-package`, and `capabilities --format json` have positive/negative CLI E2E fixtures
- [ ] producer guide, Linux actual-host evidence, and reproducibility gates exist; matching current-source macOS/Linux evidence must still aggregate before MI1-17 release completion

The first three checked items are contract decisions. The next two are implementation/public-E2E claims. The final unchecked item is deliberately a release claim and cannot be inferred from Linux-only evidence or a synthetic aggregation test.

## Source and text

- [x] source, text, and local map ranges use distinct types
- [x] no text length silently saturates
- [x] text-map segments are non-empty and cover buffers exactly on UTF-8 boundaries
- [x] identity text-map segments have equal byte lengths and byte-for-byte content
- [x] SourceCatalog and TextStore have separate owners and identifier spaces
- [x] each materialized state owns a separate immutable GeneratedTextStore/GeneratedTextBufferId namespace through Display construction
- [x] GeneratedBufferKey collection is sorted before dense GeneratedTextBufferId allocation and is independent of insertion/thread order
- [x] parsed and generated buffers are stably remapped by TextBufferId/GeneratedBufferKey to dense DisplayTextBufferId values and Display exposes only DisplayTextSpan
- [x] only note/warning `AdvisoryDiagnostic` values can accompany success
- [x] every error/fatal outcome has no success value or artifact; fatal aborts immediately and error collection stops at a safe phase boundary
- [x] only a sealed source-driven Parser can issue ValidatedParsedPackage; no feature or fixture type promotes a caller-built ParsedPackage
- [x] source/resource paths are portable and root-contained
- [x] CLI host paths remain distinct from serialized PortablePath values
- [x] file output/trace/manifest HostPath targets are pairwise non-aliasing at session creation, each write start, and final atomic publish, even with force
- [x] config project-root sentinel is a ConfigResourceRoot variant and never a PortablePath dot component
- [x] one PortablePath matching more than one admitted host root is an ambiguity error, never first-match selection
- [x] EffectiveConfig resource_roots and allowed_uri_schemes are unique UTF-8-byte-sorted sets
- [x] raw URI syntax becomes bounded allowlisted SafeUri at the syntax boundary
- [x] normalization is explicit and mapped

## Shaping and paragraph layout

- [x] Bidi level is retained through shaping and visual ordering
- [x] each broken line applies UAX #9 L1 reset, final reshape, justification, then L2 reorder
- [x] OpenType tags are exactly four printable ASCII bytes
- [x] shaper emits cluster groups
- [x] breaker never splits a cluster
- [x] items retain immutable shaped-run slices plus `TextSpan`/full `GeneratedProvenance`
- [x] generated provenance has an epoch-unique allocation-independent GeneratedBufferKey plus a local generated span
- [x] pagination fingerprints include resolved generated-text UTF-8 bytes, not only owner identity
- [x] Box/Glue/Penalty/Discretionary data and every discretionary branch's drawing content are explicit
- [x] line-shape exhaustion policy is explicit
- [x] every styleable block has unique UTF-8-byte-sorted valid class tokens
- [x] selector class components are unique UTF-8-byte-sorted; selectors accept only `block_type(.class)*` and match by class-set inclusion
- [x] style IDs are unique and extends references form a known DAG
- [x] style properties use the closed typed registry; repeated properties follow the normal important/declaration-order cascade
- [x] every text-producing block resolves a family through fully AdmittedResources, positive font size, and positive line height before shaping
- [x] `ShapeFontSelectionReceipt` binds the exact package/computed style, admitted ledger, canonical instance table, selected computed face, and LayoutEpoch; `ShapeRequest` accepts no raw instance/hash/bytes and rechecks the exact epoch
- [x] only the canonical itemizer constructs `ShapeRequest`, derives immediate logical pre/post context plus bidi/script, and rejects nonempty language/features because Profile 1.0 declares neither input
- [x] shaping cache keys include ShaperIdentity backend/version and resolved Unicode/Japanese data-table versions in addition to exact font/text/context/script/language/features
- [x] `PackageShapeTextReceipt` binds exact parsed/generated UTF-8, site owner, package-derived style owner, document fingerprint, and selected generated reference fingerprint; main/pre/post cannot mix owners
- [x] every nonempty ListItem registry entry has one canonical list-marker buffer (checked ASCII ordered number plus `.` or U+2022); marker spacing is Glue, and empty List/Table containers are rejected
- [x] Profile 1.0 ResourceCatalog family names are unique and font fallback never uses system search or collection order
- [x] DocumentFingerprint includes every ResourceCatalog declaration field, and ReferenceFingerprint JCS includes its algorithm identifier
- [x] declaration_order is derived from the declarations array and matched extends chains use it with matched-rule specificity/source_order and inheritance depth
- [x] style and page-master source_order matches dense 0-based array order
- [x] ordered lists carry an explicit positive start (default 1) and unordered lists carry null

## Fragmentation and pagination

- [x] computed `page` accepts only auto or PageName, and PageName is distinct from StyleId/MasterId
- [x] PageSelectionContext derives physical number/first/parity before master selection; PageContext is built only after one master wins
- [x] Fragmenter is re-entrant and deterministic
- [x] `More` continuation strictly advances a same-epoch global structured flow position; owner may change and fingerprints alone never prove progress
- [x] LayoutEpoch includes admitted identity/metadata but excludes Display use, subset, CID, encoding, and FrozenPdfResourcePlans
- [x] LayoutEpoch and shape-selection receipts are owned below layout/shaping by `typaxis-layout-contract`; layout only re-exports them
- [x] PaginationFingerprintRecord canonicalizes every collection with unique declared keys before JCS encoding
- [x] anchors and footnotes are returned from fragmentation; each anchor ID/owner is package-registered and bound to an exact page/frame/column/local point
- [x] state 0/selected state semantics are preserved
- [x] materialized state owns exactly the pages and overlay used to paint them; next `LayoutPassInput` combines the previous fingerprint with a sealed page/anchor/site-bound working-overlay transition
- [x] initial pagination state derives its complete canonical seed overlay only from the validated package/limits and exposes no caller-supplied store input
- [x] pagination budget fixes document/style/admitted identity but records and verifies each pass's working reference epoch; reference transition derives its owned next overlay from the exact predecessor pages/anchors/site registry rather than accepting an arbitrary store
- [x] pagination budget, materialization receipt, `LayoutPass`, transition receipt, and next pass input carry one opaque in-process session capability; another session is rejected before pass work even when its canonical state fingerprint is identical
- [x] nonblank FlowTree ends at the explicit root End sentinel, blank FlowTree uses its sole DocumentStart as terminal, `More` never targets terminal, and `Exhausted` names exactly that terminal
- [x] stable/cycle/max-pass are distinguished
- [x] only materialized states 1..pass_count enter canonical fallback scoring
- [x] `lowest_cost_then_earliest` selection and strict/no-PDF behavior are traced

## Display and resources

- [x] no PDF name/CID/object ID in Display List
- [x] every glyph run has paint and Bidi level
- [x] every internal link resolves to a destination
- [x] selected named destinations are the exact canonical selected-anchor set; missing/extra/wrong-page/frame/point entries are rejected
- [x] path state and dash invariants are validated
- [x] annotations and destinations remain inside pages
- [x] placed-anchor facts participate in pagination fingerprinting and convergence
- [x] Display page_index is dense 0-based; CLI physical page N is 1-based and maps by checked N-1
- [x] CID 0 is reserved and subset plan is unique/closed
- [x] `max_cids_per_font` is at most 65535 and enforced before allocation
- [x] font metrics and image encoding metadata are finalized
- [x] resource order is stable
- [x] admitted bytes/hash/metadata are fixed before shaping
- [x] nonempty resource admission requires one HostAdmissionContext-bound sealed root set; a source receipt from another root set is rejected
- [x] the contained opener binds no-follow opened-handle identity and an immutable/read-locked extent; post-read length mismatch fails closed
- [x] font/image count, per-resource encoded bytes, and aggregate admitted bytes are checked before read/allocation
- [x] `FrozenPdfResourcePlans` are PDF-ready but contain no backend handle, PDF name, or object ID
- [x] ResourceCollector unions repeat uses by logical ID and rejects one ID resolving to different admitted hash/metadata
- [x] font keys are `(font, admitted SHA-256, FontInstanceId)`, image keys are `(image, admitted SHA-256, ImageResourceId)`, and duplicate keys after dedupe fail
- [x] per-plan PDF object traversal follows declared type order
- [x] selected Display LayoutEpoch admitted fingerprint exactly matches the ledger used by finalization
- [x] every font plan has the complete Type0/CIDFont/descriptor/program/ToUnicode/CIDToGIDMap object blueprint and validated descriptor metrics

## PDF

- [x] duplicate object insertion leaves the builder unchanged, returns error, and stops the build
- [x] graph is frozen before serialization
- [x] Length/Filter/DecodeParms are serializer-owned
- [x] Catalog points to a parentless root Pages node; Page parent/count/cycle invariants are validated
- [x] empty documents materialize one default-master blank page; Display/PDF page collections and built page_count are nonempty
- [x] CIDToGIDMap/W/ToUnicode share one plan
- [x] subsetter rewrites and re-extracts the embedded PostScript name, and Type0/CIDFont/FontDescriptor share that receipt-bound collision-free deterministic six-letter subset name
- [x] ActualText is cluster-scoped and non-overlapping
- [x] classic xref output bound is enforced
- [x] annotation/destination coordinates are converted outside the content CTM
- [x] Catalog Names/Dests and page Annots contain every Display destination/annotation exactly once without raw actions
- [x] direct PDF values and page trees are iteratively validated at fixed depth 64 and rejected payloads are dropped iteratively

## Verification

- [x] positive and exact-code negative fixtures
- [x] effective config precedence and RFC 8785 JCS hash fixtures
- [x] configured Unicode/Japanese selectors resolve to registered table handles; manifest shaper identity is a closed registry-selection fact, engine identity is build-issued, and any actual-shaper-use claim requires a sealed runtime receipt
- [x] manifest input/font/image arrays use their unique canonical sort keys
- [x] manifest records PDF compression facts and layout termination/selection summary
- [x] ResourceLimits positivity and cross-field relations are validated exactly
- [x] Document nesting and style inheritance accept exact `max_ast_nesting_depth`, reject max+1 before recursive work, and retain distinct unknown-parent/cycle errors
- [x] exact max succeeds and max+1 fails before work; initial footnote fragment/float page are not reflow/carry and final allowed reshape failure is reported after that pass
- [x] table cells use deterministic leftmost-free placement with full row coverage and no head/body-crossing rowspan
- [x] every bounded line/lookback/footnote/column/float algorithm has exact-limit tests
- [ ] cargo check/test and public machine profile gate on every documented host target（MI0-01のmacOS reference baselineと、MI1-17 worktreeのLinux locked/static/workspace/public machine E2Eは成功済み。current-source macOS machine evidenceとLinux/macOS aggregateはCI gateで未確認）
- [x] Unicode conformance data
- [x] subset round-trip
- [x] renderer/extractor differential
- [x] fuzzing
- [x] deterministic two-build ZIP/PDF comparison, including builds from differently named checkout directories
- [x] `BuildOutputCommitContext` exists for every build, permits serializer-receipt PDF commit without a manifest target, and requires sealed built preflight whenever a manifest target is configured
- [x] manifest has only terminal built/failed states; manifest publication begins only after a session-bound `ManifestPublicationContext` exists
- [x] output/manifest contexts and serializer/sink receipts are non-cloneable; prepare and terminal commit consume their one-shot owner and reject another output session
- [x] serializer receipt carries the exact EffectiveConfig fingerprint and selected PDF graph facts; manifest-free and manifest-bound output reject another config/artifact chain
- [x] failed manifest is atomically published from a sealed session-bound preflight before its trusted record/receipt becomes visible
- [x] built output records file/stdout sink without HostPath; stdout is built only after complete write, canonical manifest bytes are atomically published after PDF, and a later manifest failure carries the already committed/emitted PDF receipt
- [x] post-publish directory-sync failure carries the visible PDF receipt or complete built/failed publication and is never reported as rollback/uncommitted
