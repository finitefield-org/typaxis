# ADR-0063: Retain and select explicit page names on source breaks

## Status

Accepted incrementally under design 28 on 2026-09-10. Extends ADR-0062 to authored
page names on standalone and nested body page-break nodes. Conflicting names inside
one parallel table and names inside footnote definitions retain explicit diagnostics.
Horizontal reflow, running regions, columns and full-book/public/managed-host/author/
human acceptance remain open.

## Decision

The successor source collector retains explicit computed page-break names with the
original node identities. The bounded registry is sorted by owner, participates in
source-flow verification and encoding, and is empty on legacy flows. Named strings
consume the source text allowance. The source-bound page plan charges live registry
records/bytes, sorting and owner lookup against the cumulative command allowances
before using the name. It keeps the original node in the same Begin/End nesting as
other body regions; no synthetic paragraph, replacement break or fake paint is made.

A page-break name selects the page on which that source node is consumed. At its End,
the enclosing used name resumes. A named break at the root therefore returns to the
default source name for following ordinary body. As with any named source transition,
a different name starts a separate page. The explicit break then advances exactly
once, including first, consecutive and trailing breaks. A final empty page retains
the name of the last consumed page while no later body item selects another name.
Thus a name change before a break and the break itself are distinct boundaries.

The mixed page selector formerly consumed any next outside-table break on the
current page after selecting body content. It now consumes that break only when its
used name matches the selected physical page. Otherwise the original break remains
pending for its own named region. Existing keep conflicts, table and definition
boundaries, exact source consumption, repeated-header behavior and footnote queues
remain authoritative. Body collection rejects named break content when it has no
bound name plan or belongs to a definition; conflicting parallel table names still
fail with their original source owner.

Legacy and unnamed flow encodings remain byte-for-byte unchanged. Explicit names
add an optional source-encoding field and are rechecked from immutable source during
flow verification. No public versioned profile or completion receipt is minted by
this internal source/pagination extension.

## Verification

Design 28 §14.197 and its progress ledger record nested and root explicit-break
fixtures, original-owner and exact/one-short budget checks, conflicting parallel and
definition cases, legacy regressions, original Harano output and independent PDF
inspection. The independent checker derives empty/text page positions from original
break order and classes, uses the declared named/default masters and original font
metrics, and rejects altered source, geometry and page sequences. Full-design
completion is not claimed.
