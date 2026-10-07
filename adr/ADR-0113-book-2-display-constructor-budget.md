# ADR-0113: Preserve display constructor reservations and verification work

## Status

Implemented incrementally under design 28 on 2026-09-22. Focused constructor
checks, agent-run CLI 266 tests, font 76 tests and workspace type checks pass
without failures or ignored tests. Independent verification covers 759 PDFs /
2,896 pages and 58 resource subsets; all 759 PDFs are byte-identical to the
preceding stage. All 90 frozen source hashes and 23 successful logs are recorded
in run-source-correspondence.json. Overall design acceptance remains open.

## Decision

Add `new_counted` entry points to the body/math and running-region display
builders. The existing constructors delegate to the same implementations.
Return the cumulative record/work prefix and every accepted constructor charge,
even when the constructor returns an error instead of an owner.

The body/math constructor observes the reserved builder record and partial
repeated-header receipt verification before propagating a validation failure.
The running-region constructor exposes its accepted font-instance storage
reservation before attempting its work reservation. If that work reservation
is rejected, retain the records without inventing accepted work. Receipt/epoch
rejection before reservation leaves only the incoming prefix. Identity checks,
error precedence, successful charges and fingerprints remain unchanged.

Connect both entry points to the private source-to-PDF driver so constructor
failure observations reach the reusable command budget. This uses the existing
single-prefix accounting and does not charge the same constructor twice when
a builder is returned successfully.

## Verification

Repeated table-header tests exercise early/middle/late constructor work failure,
exact-ceiling success, record exhaustion and retries with returned prefixes.
They run on actual native/vector/raster/header variants and unchanged Harano.
Running-region tests cover exact work, one-short work after record reservation,
retries, invalid epoch and exhausted records. Driver tests compare constructor
failure counters with attachment observations and preserve the source error.

## Remaining scope

Other constructors, source admission, shaping internals and complete command
allocation accounting remain open. This does not publish Book /2 or establish
whole-book, managed-host, controlled performance or author/human acceptance.
