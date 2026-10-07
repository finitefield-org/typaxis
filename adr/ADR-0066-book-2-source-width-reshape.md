# ADR-0066: Rebind source widths through actual shaping feedback

## Status

Accepted incrementally under design 28 on 2026-09-10. Extends ADR-0065 from a
single prepared item owner to actual repeated shaping of the same source flow.
Physical-page width feedback and full-design acceptance remain open.

## Decision

Retain provisional width assignments against the exact immutable Book-2 source
flow. Borrow the dense width slices rather than retaining an obsolete shaped
paragraph. The constructor checks paragraph cardinality without allocation; each
projection validates slot cardinality and rebinds to the new prepared item owner.
A different flow, including rebuilt generated page labels, requires new assignments.

Before binding, compare original source scalars with prepared text units in order.
For vector/native-math atoms and soft/hard breaks, compare original owner and span;
break kind must also match. Anchors and inline container boundaries consume no
logical width slot. Empty paragraphs retain one width slot and one nonpainting line.
No source item may be omitted, added or reordered to obtain a different width.

Preflight binding and width records with the caller's prior records and retained
native computations before allocating the binding vector. The common projection
charges those records once. Source paragraph/site/scalar visits consume the same
remaining candidate work as line selection. Each shaping pass creates new exact
bindings; initial selection and all rebreaks share the existing cumulative work
and reshape-pass allowances. Fixed-width callers retain the original route.

Use actual selected byte boundaries as the next shaping context and compare actual
selected fingerprints for stability. Reuse original native computations. Retained
measurement frames remain containing envelopes, and each selected width must fit
its envelope. The ordinary private PDF driver does not yet derive these assignments
from physical-page placement. Its HorizontalReflow guard remains intact.

## Evidence

Design 28 §14.200 and its progress ledger record controlled text, unchanged Harano,
multiscalar Japanese graphemes, original native math, vector/math-vector atoms,
soft/hard breaks, anchors, empty source and generated footnote markers in tables and
lists. Dense and sparse assignments exercise fixed-width fallback. Tests compare
source ranges and original glyph identity, verify actual stable shaping, and enforce
exact/one-short work, records and reshape passes. Existing PDF output is regression
checked separately; no variable-physical-width PDF acceptance is implied.
