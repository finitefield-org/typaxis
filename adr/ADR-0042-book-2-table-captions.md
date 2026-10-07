# ADR-0042: Book-2 source table captions

## Status

Accepted for implementation on 2026-09-09 under
[design 28](../docs/28-vmb-book-production-compatibility.md).
This is a private contract-1.5 extension. Carrier acceptance is separate from
source-flow, pagination, PDF, VMB producer and public-profile acceptance.

## Decision

An optional `caption` member on a Book-2 `table` retains a nonempty array of
ordinary recursive Book-2 blocks. Omission preserves the existing uncaptioned
table. Explicit null, an empty array and unknown block/inline fields are invalid.
Caption blocks retain their original node IDs, spans, classes, languages, rich
inline children, math and references. They belong to the table and precede its
head/body rows in source traversal. They are not table cells or repeated headers.
Array order is author order; JSON member order does not determine traversal.

The caption's block and inline nodes consume the same AST node/depth allowance
as other owned subflows, beginning one level below the table. No synthetic source
node or extra per-table string is introduced. Existing recursive math-source and
vector-wire checks must visit captions on decode and typed re-encode. Canonical
round trips retain the complete table and caption, including nested tables.

The eventual source/PDF connection must place the caption and rows with the
common flow, using actual measured content. Its PDF structure is Table with a
Caption child followed by THead/TBody, retaining TR/TH/TD and cell associations.
Caption text must not be duplicated on continuation pages as a table header.
The VMB producer must retain both authored Title and Caption when both exist,
with a single explicit number prefix and ADR-0041 binding to the real displayed
number. It must not use Caption as a fallback that discards an existing Title.

## Isolation and rollout

The sealed carrier trait enables table captions only for Book-2. Legacy typed
deserialization rejects any present caption, including null or an empty array;
typed serialization cannot emit one. An absent caption has unchanged legacy
canonical bytes, including in builds with staging enabled. Public dispatch and
profile identifiers are not changed by this carrier.

The existing frozen-carrier validator sees a temporary view with caption blocks
before the corresponding uncaptioned table, just as it already uses a temporary
view for semantic containers. It validates the unchanged block/inline fields.
The retained wire and canonical result never use that flattened view; the full
original tree remains subject to the successor's node/depth and shape checks.

Rollout uses explicit guards at the first unsupported stage. Source lowering
initially rejected captioned tables. The source connection now retains the
caption in the typed domain, original-source validation, style/language,
references, numbered-label ancestry and nested event flow. A bounded event range
belongs to the outer table even when the caption contains another table. The
source structure visitor emits Caption before THead/TBody without cell attributes.
These source views do not authorize layout or PDF output.

Common frames now retain the table content width for captions, and the shared
collector closes each caption at its source event boundary before the rows.
Caption children use the common measured leaf/nested-table extents. Their height
precedes row offsets and contributes once to total table height; parallel cell
heights are not added as consecutive content. Retained caption/content records
consume the existing record and canonical-byte budgets.

The table-search kernel now consumes measured caption leaves before rows.
Continuation retains whether rows have started, separately from whether this is
the first table fragment. Caption-only pages do not paint or consume headers;
the first row fragment paints the original header after any remaining caption,
and later row fragments repeat only the header. Caption leaves have no cell role.
Original caption/header/body ranges feed the common footnote-demand and source
closure paths exactly once. Candidate records, work and fingerprints include
the caption interval and header state; uncaptioned encodings remain unchanged.

The former PendingRegion("table_caption_pagination") guard is removed with
actual driver PDFs and independent structure/extraction evidence in design
14.172. Caption forced breaks are subsequently connected by ADR-0044 and design
14.175, including source cursors and blank pages. Nested caption tables still
reject with `nested_table_breaks`; header/body-cell breaks and footnote-definition
tables retain their existing restrictions. ADR-0045 subsequently connects body
cell breaks in tables without rowspans. The private VMB exporter now preserves both
Title and Caption through rich paragraphs, with one explicit number prefix and
source-bound NumberRanges metadata. A number-only caption has generated
provenance. Omitted captions retain the former wire bytes; legacy export rejects
the new fields/settings. Original Harano source-to-PDF evidence is recorded in
design 14.173. No receipt or PDF may silently ignore unsupported caption content.
Legacy profile domain validation rejects captions regardless of this staging
connection. Publication and original full-book gates remain separate.
