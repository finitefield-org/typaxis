# ADR-0105: Keep CFF subset output stages and measured byte limits

## Status

Accepted incrementally under design 28 on 2026-09-20. User-run regression passes:
253 CLI tests, 75 font tests, one legacy diagnostic test and one public diagnostic
test covering seven cases, all with zero failures or ignored tests. Independent
verification passes for 759 PDFs and 58 resource subsets; all 759 PDFs remain
byte-identical to the preceding stage. All 56 frozen source hashes match the
executed sources. Overall design acceptance remains open.

## Decision

The legacy /1 writer records a bounded FontSubsetStage in FontFailureContext.
It distinguishes PostScript naming, glyph storage and bounds, charstring encoding,
global bounds, CFF/cmap/head/horizontal metrics/maxp/name encoding, SFNT size and
writing, and PDF metrics. The existing public context note includes subset_stage
only when this writer has observed a stage. Contract-1.4 JSON fields do not change.

These are output-generation stages, not locations in the source font. Preserve
the actual original GID while encoding its bounds or program, then clear it
before aggregate tables. Retain face and embedding permission, but do not attach
source table tags, byte offsets or Type2 operators to generated data. Failed
selection before writing has no output stage.

At the existing final SFNT size check, record the configured byte limit and exact
required size, including directory and four-byte table padding. Other allocation
or arithmetic failures sharing SubsetByteLimit do not invent a configured limit
or observed size. The broad error kind, acceptance boundary, successful bytes,
receipt identity and work accounting remain unchanged. This does not introduce
an allocation preflight for the /1 writer.

## Verification and remaining scope

Font tests compare the reported required size with a successful subset and test
one byte below, exactly at and above that size. Repeated output failure keeps
evaluated glyphs and does not charge their evaluation again. Separate cases
cover an unrepresentable subset name, empty selected cmap and a valid source
contour whose outward-rounded bound does not fit the output's i16 field.

The public check/build fixture adds the same contour and retains the existing
six cases. For the byte-limit case, a successful build of the same package saves
its embedded OpenType program and PDF. The independent verifier checks their
correspondence, SFNT checksum and padded table sizes before comparing the
diagnostic's observed value. It also uses FontTools to evaluate the extreme
source contour. Public verification passes for seven cases and rejects 84
alterations. The observed size is 764 bytes and matches the actual embedded
OpenType program in the successful reference build.

Detailed /2 subset output diagnostics, exact generated-field locations where
available, command-wide budgets, public Book /2, full-book/performance/managed-host
and author/human acceptance remain open. Do not infer any of these from /1 tests.
