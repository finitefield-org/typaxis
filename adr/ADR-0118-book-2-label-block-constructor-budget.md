# ADR-0118: Preserve equation-label and vector-block constructor charges

## Status

Implemented and locally verified under design 28 on 2026-09-22. Overall
design acceptance remains open.

## Decision

Add counted staging entry points for authored equation-number shaping and
vector-block layout. Existing entry points delegate to the same implementation.
Publish the existing accepted record reservations before allocation or later
style, shaping, geometry and canonical-buffer checks can fail. Rejected record
increments remain uncharged. Validation order, typed errors, fingerprints and
successful accounting remain unchanged.

Equation labels start from the caller's prior history and retain their owner,
shape and accepted run/glyph/cluster records. A source without labels returns
None without creating a label owner. Failed operations inside the shared shaper
remain outside this observation; this does not claim complete shaping work or
allocation accounting.

Vector blocks preserve the maximum shared history from lines and label inputs,
then independently retained label storage, then their own accepted bound. The
observer initially retains input histories even if receipt verification fails.
Absent blocks do not reserve a new block owner. Geometry rejection after the
accepted bound retains that bound.

The ordinary PDF and header-catalog drivers recover observations before
propagating the original stage/cause. The ordinary driver keeps passing its
original shared prefix to subsequent constructors, so reporting an intermediate
observation does not independently recharge retained labels or blocks.

## Verification

Real TrueType and original Harano inputs exercise no labels/blocks, single and
multiple labels, every label record boundary, exact record limits, invalid
limits, missing style, post-shaping spool rejection and label/formula overlap.
Counted results are compared with the old API and successful fingerprints.
Ordinary-driver failure records are compared with independent constructor
observations. Header preparation exercises the same typed failures and retained
retry budgets before invoking any catalog consumer. Full regression and PDF
correspondence evidence belongs to
`workspace/target/vmb-design/20260922/label-block-constructor-budget/`.

Workspace checking, 278 Book /2 CLI tests, 27 shaping tests (25 default plus
two explicitly enabled original-Harano cases), 69 layout tests, 104 pagination
tests, 76 font tests and public/legacy CFF diagnostics passed. Shaping/layout
doc-tests also passed. Independent validation accepted 759 PDFs / 2,896 pages
and 58 subsets; all 759 PDFs were byte-identical to the preceding stage. The
correspondence report binds 133 frozen sources, 19 successful logs and the
supplemental shaping runner/command; test names prove no missing/duplicate
shaping cases.

## Remaining scope

Line-context capture, footnote-line projection, initial shaping/admission,
unmetered internal work and complete command allocation remain open. This does
not publish Book /2 or prove original whole-book, managed-host, controlled
performance, author or visual acceptance. Missing Speech/SemanticRef data is
user-confirmed uncreated.
