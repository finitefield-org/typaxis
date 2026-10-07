# ADR-0106: Preserve CFF /2 subset failures through the book resource owner

## Status

Accepted incrementally under design 28 on 2026-09-20. User-run regression passes:
254 CLI tests, 76 font tests and the dedicated subset, public /1 and legacy
diagnostic tests, all with zero failures or ignored tests. Independent checks
pass for 759 PDFs and 58 resource subsets; all 759 PDFs remain byte-identical to
the preceding stage. All 62 frozen source hashes match the executed sources.
Overall design acceptance remains open.

## Decision

Add write_prepared_subset_detailed_with_charge to the sealed /2 subset session.
The existing broad method delegates and returns the same Cff1Error. Keep the
existing closure verification, callback ordering, selected-glyph cache, output
bytes and receipt rules. Neither writer entry point evaluates missing glyphs.

Record FontSubsetStage at the actual output operation. Distinguish cumulative
canonical charstring size from final padded SFNT size; only those internal limit
checks populate the configured limit and observed size. Arithmetic and allocation
failure or caller charge rejection never acquire invented internal limit values.
Record original GID during glyph work and clear it before aggregate tables.
Derive original FD and OS/2 permission once on failure from admitted font facts.
Do not attach source byte positions or source table tags to generated data.

The book resource writer uses the detailed method and retains the closure's
FontFaceId with Cff1Failure in FontDetailed. Standard Error::source preserves this
failure through the existing font-program and PDF-driver chain. Caller-owned
record/spool/work rejection keeps its original BookV2FontSelectionError and does
not become a per-font byte-limit error. The existing session is restored on both
success and failure.

## Verification and remaining scope

The font test uses unchanged original Harano and compares successful bytes,
receipt identity and the complete charge sequence between both writer APIs.
It covers the early charstring cap, final size minus one, exact size, one above,
18 caller-rejection cases across six stages, missing evaluated glyphs and a
foreign limit receipt. Evaluation work and cache remain unchanged by writing.

A new private-driver test prepares the same source through admission, shaping,
layout and resource finalization. It saves a successful reference PDF/font and
checks both size failures through the real error chain with FontFaceId. Exact
final size must still permit PDF generation. A FontTools verifier compares source
and subset glyph bounds/advances, original FDSelect, canonical .notdef byte length,
complete SFNT size and diagnostic notes. The driver and independent verifier pass
both failure cases and reject all 24 mutations. The observed charstring size is
77 bytes at original GID 0 / FD 5; final SFNT size is 1,180 bytes. Both retain
FontFaceId 0 through a four-link error source chain. Exact final size also permits
PDF generation with the same embedded font bytes.

This does not publish Book /2 or change contract/profile identifiers. Exact
generated-field locations, command-wide budgets, whole-book/public/performance/
managed-host and author/human acceptance remain separate open gates.
