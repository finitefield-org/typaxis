# Implementation checklist

## Source and text

- [ ] source, text, and local map ranges use distinct types
- [ ] no text length silently saturates
- [ ] text-map segments are non-empty and cover buffers exactly on UTF-8 boundaries
- [ ] identity text-map segments have equal byte lengths and byte-for-byte content
- [ ] SourceCatalog and TextStore have separate owners and identifier spaces
- [ ] each materialized state owns a separate immutable GeneratedTextStore/GeneratedTextBufferId namespace through Display construction
- [ ] GeneratedBufferKey collection is sorted before dense GeneratedTextBufferId allocation and is independent of insertion/thread order
- [ ] parsed and generated buffers are stably remapped by TextBufferId/GeneratedBufferKey to dense DisplayTextBufferId values and Display exposes only DisplayTextSpan
- [ ] only note/warning `AdvisoryDiagnostic` values can accompany success
- [ ] every error/fatal outcome has no success value or artifact; fatal aborts immediately and error collection stops at a safe phase boundary
- [ ] only a sealed source-driven Parser can issue ValidatedParsedPackage; no feature or fixture type promotes a caller-built ParsedPackage
- [ ] source/resource paths are portable and root-contained
- [ ] CLI host paths remain distinct from serialized PortablePath values
- [ ] file output/trace/manifest HostPath targets are pairwise non-aliasing at session creation, each write start, and final atomic publish, even with force
- [ ] config project-root sentinel is a ConfigResourceRoot variant and never a PortablePath dot component
- [ ] one PortablePath matching more than one admitted host root is an ambiguity error, never first-match selection
- [ ] EffectiveConfig resource_roots and allowed_uri_schemes are unique UTF-8-byte-sorted sets
- [ ] raw URI syntax becomes bounded allowlisted SafeUri at the syntax boundary
- [ ] normalization is explicit and mapped

## Shaping and paragraph layout

- [ ] Bidi level is retained through shaping and visual ordering
- [ ] each broken line applies UAX #9 L1 reset, final reshape, justification, then L2 reorder
- [ ] OpenType tags are exactly four printable ASCII bytes
- [ ] shaper emits cluster groups
- [ ] breaker never splits a cluster
- [ ] items retain immutable shaped-run slices plus `TextSpan`/full `GeneratedProvenance`
- [ ] generated provenance has an epoch-unique allocation-independent GeneratedBufferKey plus a local generated span
- [ ] pagination fingerprints include resolved generated-text UTF-8 bytes, not only owner identity
- [ ] Box/Glue/Penalty/Discretionary data and every discretionary branch's drawing content are explicit
- [ ] line-shape exhaustion policy is explicit
- [ ] every styleable block has unique UTF-8-byte-sorted valid class tokens
- [ ] selector class components are unique UTF-8-byte-sorted; selectors accept only `block_type(.class)*` and match by class-set inclusion
- [ ] style IDs are unique and extends references form a known DAG
- [ ] style properties use the closed typed registry; repeated properties follow the normal important/declaration-order cascade
- [ ] every text-producing block resolves a family through fully AdmittedResources, positive font size, and positive line height before shaping
- [ ] `ShapeFontSelectionReceipt` binds the exact package/computed style, admitted ledger, canonical instance table, selected computed face, and LayoutEpoch; `ShapeRequest` accepts no raw instance/hash/bytes and rechecks the exact epoch
- [ ] only the canonical itemizer constructs `ShapeRequest`, derives immediate logical pre/post context plus bidi/script, and rejects nonempty language/features because Profile 1.0 declares neither input
- [ ] shaping cache keys include ShaperIdentity backend/version and resolved Unicode/Japanese data-table versions in addition to exact font/text/context/script/language/features
- [ ] `PackageShapeTextReceipt` binds exact parsed/generated UTF-8, site owner, package-derived style owner, document fingerprint, and selected generated reference fingerprint; main/pre/post cannot mix owners
- [ ] every nonempty ListItem registry entry has one canonical list-marker buffer (checked ASCII ordered number plus `.` or U+2022); marker spacing is Glue, and empty List/Table containers are rejected
- [ ] Profile 1.0 ResourceCatalog family names are unique and font fallback never uses system search or collection order
- [ ] DocumentFingerprint includes every ResourceCatalog declaration field, and ReferenceFingerprint JCS includes its algorithm identifier
- [ ] declaration_order is derived from the declarations array and matched extends chains use it with matched-rule specificity/source_order and inheritance depth
- [ ] style and page-master source_order matches dense 0-based array order
- [ ] ordered lists carry an explicit positive start (default 1) and unordered lists carry null

## Fragmentation and pagination

- [ ] computed `page` accepts only auto or PageName, and PageName is distinct from StyleId/MasterId
- [ ] PageSelectionContext derives physical number/first/parity before master selection; PageContext is built only after one master wins
- [ ] Fragmenter is re-entrant and deterministic
- [ ] `More` continuation strictly advances a same-epoch global structured flow position; owner may change and fingerprints alone never prove progress
- [ ] LayoutEpoch includes admitted identity/metadata but excludes Display use, subset, CID, encoding, and FrozenPdfResourcePlans
- [ ] LayoutEpoch and shape-selection receipts are owned below layout/shaping by `typaxis-layout-contract`; layout only re-exports them
- [ ] PaginationFingerprintRecord canonicalizes every collection with unique declared keys before JCS encoding
- [ ] anchors and footnotes are returned from fragmentation; each anchor ID/owner is package-registered and bound to an exact page/frame/column/local point
- [ ] state 0/selected state semantics are preserved
- [ ] materialized state owns exactly the pages and overlay used to paint them; next `LayoutPassInput` combines the previous fingerprint with a sealed page/anchor/site-bound working-overlay transition
- [ ] initial pagination state derives its seed overlay only from the validated package/limits, rejects unsupported nonempty generated-site registries, and exposes no caller-supplied store input
- [ ] pagination budget fixes document/style/admitted identity but records and verifies each pass's working reference epoch; reference workspace rejects nonempty generated-site transition instead of accepting an arbitrary store
- [ ] pagination budget, materialization receipt, `LayoutPass`, transition receipt, and next pass input carry one opaque in-process session capability; another session is rejected before pass work even when its canonical state fingerprint is identical
- [ ] nonblank FlowTree ends at the explicit root End sentinel, blank FlowTree uses its sole DocumentStart as terminal, `More` never targets terminal, and `Exhausted` names exactly that terminal
- [ ] stable/cycle/max-pass are distinguished
- [ ] only materialized states 1..pass_count enter canonical fallback scoring
- [ ] `lowest_cost_then_earliest` selection and strict/no-PDF behavior are traced

## Display and resources

- [ ] no PDF name/CID/object ID in Display List
- [ ] every glyph run has paint and Bidi level
- [ ] every internal link resolves to a destination
- [ ] selected named destinations are the exact canonical selected-anchor set; missing/extra/wrong-page/frame/point entries are rejected
- [ ] path state and dash invariants are validated
- [ ] annotations and destinations remain inside pages
- [ ] placed-anchor facts participate in pagination fingerprinting and convergence
- [ ] Display page_index is dense 0-based; CLI physical page N is 1-based and maps by checked N-1
- [ ] CID 0 is reserved and subset plan is unique/closed
- [ ] `max_cids_per_font` is at most 65535 and enforced before allocation
- [ ] font metrics and image encoding metadata are finalized
- [ ] resource order is stable
- [ ] admitted bytes/hash/metadata are fixed before shaping
- [ ] nonempty resource admission requires one HostAdmissionContext-bound sealed root set; a source receipt from another root set is rejected
- [ ] the contained opener binds no-follow opened-handle identity and an immutable/read-locked extent; post-read length mismatch fails closed
- [ ] font/image count, per-resource encoded bytes, and aggregate admitted bytes are checked before read/allocation
- [ ] `FrozenPdfResourcePlans` are PDF-ready but contain no backend handle, PDF name, or object ID
- [ ] ResourceCollector unions repeat uses by logical ID and rejects one ID resolving to different admitted hash/metadata
- [ ] font keys are `(font, admitted SHA-256, FontInstanceId)`, image keys are `(image, admitted SHA-256, ImageResourceId)`, and duplicate keys after dedupe fail
- [ ] per-plan PDF object traversal follows declared type order
- [ ] selected Display LayoutEpoch admitted fingerprint exactly matches the ledger used by finalization
- [ ] every font plan has the complete Type0/CIDFont/descriptor/program/ToUnicode/CIDToGIDMap object blueprint and validated descriptor metrics

## PDF

- [ ] duplicate object insertion leaves the builder unchanged, returns error, and stops the build
- [ ] graph is frozen before serialization
- [ ] Length/Filter/DecodeParms are serializer-owned
- [ ] Catalog points to a parentless root Pages node; Page parent/count/cycle invariants are validated
- [ ] empty documents materialize one default-master blank page; Display/PDF page collections and built page_count are nonempty
- [ ] CIDToGIDMap/W/ToUnicode share one plan
- [ ] subsetter rewrites and re-extracts the embedded PostScript name, and Type0/CIDFont/FontDescriptor share that receipt-bound collision-free deterministic six-letter subset name
- [ ] ActualText is cluster-scoped and non-overlapping
- [ ] classic xref output bound is enforced
- [ ] annotation/destination coordinates are converted outside the content CTM
- [ ] Catalog Names/Dests and page Annots contain every Display destination/annotation exactly once without raw actions
- [ ] direct PDF values and page trees are iteratively validated at fixed depth 64 and rejected payloads are dropped iteratively

## Verification

- [ ] positive and exact-code negative fixtures
- [ ] effective config precedence and RFC 8785 JCS hash fixtures
- [ ] configured Unicode/Japanese selectors resolve to registered table handles; manifest shaper identity is a closed registry-selection fact, engine identity is build-issued, and any actual-shaper-use claim requires a sealed runtime receipt
- [ ] manifest input/font/image arrays use their unique canonical sort keys
- [ ] manifest records PDF compression facts and layout termination/selection summary
- [ ] ResourceLimits positivity and cross-field relations are validated exactly
- [ ] Document nesting and style inheritance accept exact `max_ast_nesting_depth`, reject max+1 before recursive work, and retain distinct unknown-parent/cycle errors
- [ ] exact max succeeds and max+1 fails before work; initial footnote fragment/float page are not reflow/carry and final allowed reshape failure is reported after that pass
- [ ] table cells use deterministic leftmost-free placement with full row coverage and no head/body-crossing rowspan
- [ ] every bounded line/lookback/footnote/column/float algorithm has exact-limit tests
- [ ] cargo check/test
- [ ] Unicode conformance data
- [ ] subset round-trip
- [ ] renderer/extractor differential
- [ ] fuzzing
- [ ] deterministic two-build ZIP/PDF comparison, including builds from differently named checkout directories
- [ ] `BuildOutputCommitContext` exists for every build, permits serializer-receipt PDF commit without a manifest target, and requires sealed built preflight whenever a manifest target is configured
- [ ] manifest has only terminal built/failed states; manifest publication begins only after a session-bound `ManifestPublicationContext` exists
- [ ] output/manifest contexts and serializer/sink receipts are non-cloneable; prepare and terminal commit consume their one-shot owner and reject another output session
- [ ] serializer receipt carries the exact EffectiveConfig fingerprint and selected PDF graph facts; manifest-free and manifest-bound output reject another config/artifact chain
- [ ] failed manifest is atomically published from a sealed session-bound preflight before its trusted record/receipt becomes visible
- [ ] built output records file/stdout sink without HostPath; stdout is built only after complete write, canonical manifest bytes are atomically published after PDF, and a later manifest failure carries the already committed/emitted PDF receipt
- [ ] post-publish directory-sync failure carries the visible PDF receipt or complete built/failed publication and is never reported as rollback/uncommitted
