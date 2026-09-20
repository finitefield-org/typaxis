# ADR-0110: Retain failed page-search work and begun selection passes

## Status

Implemented incrementally under design 28 on 2026-09-20. Focused driver tests and
workspace type checks pass. Agent-run CLI 262 tests, font 76 tests and all dedicated
checks pass without failures or ignored tests. Independent verification covers 759
PDFs / 2,896 pages, with all 759 byte-identical to the preceding stage. All 80 frozen
source hashes and 19 successful logs are recorded in run-source-correspondence.json.
Overall design acceptance remains open.

## Decision

Add counted page convergence to the shared stability kernel. Count a pass only
after its record charge succeeds and before selection begins. Selection and
comparison failures preserve all begun passes. The existing entry point delegates
without changing success behavior or legacy limits; fewer than two available
passes still reject before work.

Expose counted mixed-page stability on the Book /2 search owner. Its existing
work and record counters already retain partial search, placement, source closure,
width feedback and math-terminal work. The private driver now commits begun
passes before propagating stability errors and recovers these search counters
when no downstream successful observation has already accounted for them.

Successful width/header retries and completed PDF callbacks retain their existing
accounting. A later callback failure does not double-count search work. Search
record high water is preserved on failure. Keep original stage errors and causes.

## Verification and remaining scope

The shared kernel test injects first/second pass-charge rejection, first/second
selection failure and comparison failure, checking exact begun passes and source
owners. A real narrow-height body test independently measures preparation, line
and failed search work, compares their sum with driver counters, retries on one
owner, and checks exact work exhaustion without refunding begun passes.

Search-constructor failures and downstream-only display/PDF work still require
their own failure observations. Shaping internals, all initial frame construction,
source admission and complete command allocation accounting remain open, as do
public Book /2, whole-book, performance, managed-host and author/human acceptance.
