# ADR-0040: Book-2 authored description lists

## Status

Accepted for implementation on 2026-09-09 under
[design 28](../docs/28-vmb-book-production-compatibility.md). This is an
unpublished contract-1.5 decision, not a production profile or PDF support claim.

## Context

The original fractions book has 30 description lists. VMB retains each item's
`Term []Inline` separately from its ordered `Blocks []RenderBlock`. The existing
`list` carrier has only `ordered`, `start` and generated bullet/number semantics.
Converting a description list into that carrier would discard its authored term
boundary and could introduce a marker the author did not supply.

## Decision

Contract 1.5 adds a closed `description_list` block:

- `kind`, `node_id`, `span`, `classes`, nonempty `items`, optional `language`.
- Each item has `node_id`, `span`, `term`, nonempty `blocks`, optional `language`.
- The term has its own `node_id`, `span`, `classes`, nonempty `children` inline
  array and optional `language`. It has no `kind` member: it is an authored
  description term, not a paragraph substituted for a term.

`ordered`, `start`, generated marker strings, arbitrary extra fields, absent or
null terms and empty term/definition arrays are invalid. Inline decoration,
links, references, native math, precomposed vectors and their original source
bindings remain typed inline content. A nonempty array alone does not establish
nonempty authored content; recursive content and source validation remain syntax
obligations. Empty descendants must never cause silent removal of an item.

The source preorder is list, then each item, its term and term inline descendants,
then its definition blocks and descendants. All original owners, spans, classes,
language inheritance and text/math provenance must survive canonicalization,
source validation, flow, selected fragments and structure. Definition blocks
permit the general recursive grammar, including nested description lists, other
lists, tables, semantic containers, captions and footnotes. Header/footer grammar
is unchanged and does not accept description lists.

The planned standard PDF structure is `L` containing `LI`, with the authored term
under `Lbl` and the definition under `LBody`. Term content must use actual source
paint/MCIDs, including wrapped text, math, links and empty physical lines where
applicable. No bullet/number generator may replace it. Source style, term/body
layout, splitting, navigation and PDF implementation require separate evidence.

## Shared carrier and isolation

The shared generic block enum exposes the additional variant only in tests or
`book-v2-staging`. Its sealed semantic-kind trait independently controls whether
description items can be serialized or deserialized. Contract 1.4 rejects the
variant even when staging is compiled, including an empty item array and a
manually constructed legacy DTO. Ordinary 1.4 lists retain their exact shape and
canonical bytes; normal public contract/profile dispatch stays unchanged.

The temporary 1.3 validation view can represent the term using its existing
inline-bearing paragraph shape and the list using its recursive list shape, to
check unchanged supporting fields. This view is never retained or exported and
grants no source or layout authority. The original successor wire and canonical
bytes retain `description_list`, its term and all owners. AST charges count each
item and term separately, including every inline descendant at its real depth.

## Required evidence

- Lossless canonical decode/encode in every general recursive slot, including
  nested descriptions, native/vector terms, language and original bindings.
- Missing/null/unknown/duplicate fields and mutated typed input fail; term and
  definition counts/depth obey exact limits; page regions reject this grammar.
- Legacy raw DTO serialization/deserialization and 1.4/public dispatch reject
  description lists; existing package/syntax/public regressions remain green.
- Before treating descriptions as supported output, demonstrate actual source
  validation, term/body flow, line shaping, splitting and PDF structure/extraction
  from the original authored terms. Carrier acceptance alone is insufficient.
- Before publication, complete the formal exporter, original full book and all
  design-28 resource, independent verification and human acceptance gates.
