# Implementation checklist

## Source and text

- [ ] source, text, and local map ranges use distinct types
- [ ] no text length silently saturates
- [ ] text-map segments are non-empty and cover buffers exactly on UTF-8 boundaries
- [ ] source/resource paths are portable and root-contained
- [ ] normalization is explicit and mapped

## Shaping and paragraph layout

- [ ] Bidi level is retained through shaping and visual ordering
- [ ] OpenType tags are exactly four printable ASCII bytes
- [ ] shaper emits cluster groups
- [ ] breaker never splits a cluster
- [ ] Box/Glue/Penalty/Discretionary data is explicit
- [ ] line-shape exhaustion policy is explicit

## Fragmentation and pagination

- [ ] PageContext flags are derived
- [ ] Fragmenter is re-entrant and deterministic
- [ ] continuation advances or exhausts; zero progress is rejected
- [ ] anchors and footnotes are returned from fragmentation
- [ ] state 0/selected state semantics are preserved
- [ ] stable/cycle/max-pass are distinguished

## Display and resources

- [ ] no PDF name/CID/object ID in Display List
- [ ] every glyph run has paint and Bidi level
- [ ] every internal link resolves to a destination
- [ ] path state and dash invariants are validated
- [ ] annotations and destinations remain inside pages
- [ ] CID 0 is reserved and subset plan is unique/closed
- [ ] font metrics and image encoding metadata are finalized
- [ ] resource order is stable

## PDF

- [ ] duplicate object insertion preserves first value
- [ ] graph is frozen before serialization
- [ ] Length/Filter/DecodeParms are serializer-owned
- [ ] Catalog points to a parentless root Pages node; Page parent/count/cycle invariants are validated
- [ ] CIDToGIDMap/W/ToUnicode share one plan
- [ ] ActualText is cluster-scoped and non-overlapping
- [ ] classic xref output bound is enforced

## Verification

- [ ] positive and exact-code negative fixtures
- [ ] cargo check/test
- [ ] Unicode conformance data
- [ ] subset round-trip
- [ ] renderer/extractor differential
- [ ] fuzzing
- [ ] deterministic two-build ZIP/PDF comparison, including builds from differently named checkout directories
