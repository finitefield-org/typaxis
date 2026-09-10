# ADR-0065: Bind variable inline widths to original source starts

## Status

Accepted incrementally under design 28 on 2026-09-10. This establishes the line
selection primitive needed by width-dependent page feedback. Actual physical page
reflow, running regions, columns and full-book/public acceptance remain open.

## Decision

Bind a provisional dense width assignment to the exact prepared paragraph item
owner. There is one positive width for every possible logical-unit start, including
unreachable starts inside an indivisible cluster, and one slot for an empty source.
Re-itemization or reshaping requires a newly bound assignment. The width for a
candidate is determined by its original start, not by an unstable selected-line
ordinal. A width change inside a candidate does not itself force a break.

Use the existing minimum-demerit kernel for both fixed and source-dependent widths.
Preserve mandatory boundaries, whole clusters, atomic vector/native-math geometry,
nonnegative advances and overhang compensation. Never skip source to reach a wider
region. Retain the selected width on every resulting line. Fixed-width canonical
bytes remain unchanged; variable-width fingerprints use a distinct algorithm and
include the complete dense assignment, including unselected starts.

Charge width visits to the caller's existing work allowance before allocating the
search arena. Book-2 projection validates exact paragraph-owner identity and charges
profile records to the remaining document allowance. It projects the original
shaped glyphs and source spans through selected lines, and requires each selected
width to fit the declared containing envelope. This result has no selected physical
frames. The page-plan HorizontalReflow guard remains until source-to-page feedback,
reshaping, tables/notes and actual frame placement are connected and verified.

## Evidence

Design 28 §14.199 and its progress ledger record exhaustive comparison of 729 width
assignments against all 32 complete partitions of a six-unit source, exact/one-short
work and selection budgets, indivisible clusters, mandatory breaks, vector overhang,
empty source and fingerprints of unreachable starts. Actual controlled and unchanged
Harano shaping selects different line ends while retaining original glyph pointers
and source. Foreign preparations, malformed profiles and exhausted shared budgets
are rejected. Existing fixed-width PDFs remain byte-identical.

This proves source-dependent line selection and projection, not physical page reflow.
Variable-width native-math projection is not separately claimed by these fixtures.
