# ADR-0086: Resolve header math terminals through each actual measurement

## Status

Accepted incrementally under design 28 on 2026-09-10. Variant-bearing source
closures can issue math terminals after independent physical width validation.
Display/font/resource assembly and private PDF driver integration remain open.

## Decision

Replace the blanket variant math-terminal rejection with actual table source-frame
and paragraph/block width validation. Provisional base body widths still fail with
WidthMismatch. For each placed fragment, resolve its exact source-closed flow
before reading a selected paragraph line, inline math receipt, vector block or
native display block. Keep global terminal fragment identity and physical origins,
baselines, viewports, original cell role and explicit repeated status.

Semantic and repeated formula counts must equal the source closure's counts.
Native computations and vector bindings remain exact original receipt owners;
repeated paint does not recompute or consume the original formula twice. Equation
number records retain the actual placed geometry, including header repetitions.

For variant-bearing results append a tagged canonical owner section after existing
formula and equation-number records. Its header is 40 bytes (SHA-256 domain tag
and u64 count); each 180-byte entry carries physical page, local fragment and
variant global item indexes, plus exact header, selected lines, block layout,
vector bindings and native computation fingerprints. Include every header fragment,
even those without a formula. No-header bytes remain identical. The tag is
`typaxis.book-2-math-terminal-header-owners/1`.

Calculate the full extension bound before allocation and include it in cumulative
spool limits. Charge per-fragment owner lookup, owner-section traversal and the
complete canonical hash. Original native computation spool remains a mandatory
lower bound; seed-set compatibility requires it to be shared. Exact/one-short
work, record and spool tests include all new proof and encoding work.

Keep display construction explicitly rejected with PendingHeaderVariants until
its text, markers, figures and font/resource consumers support the resolved flows.
A valid math-terminal result alone must not enable a base-only glyph/PDF path.

## Evidence and remaining work

Design 28 §14.220 and the progress ledger record native and vector formula fixtures
in body and notes, with both directions of differing source line partitions.
A retained legal boundary before the formula produces a separate two-line header
at the same valid physical width; each can serve as base or repeated variant.
Verify actual inline/native/block owners, physical terminal offsets, repeated
counts, vector figures, equation numbers and the full canonical extension with an
independent decoder. The existing sixteen varying-width cases continue to reject
unconverged base geometry with WidthMismatch.

These are terminal/geometry tests, not new variant PDF/font-resource acceptance.
The dedicated math fixtures use controlled fonts and constant valid page widths;
actual different-width PDF convergence, original Harano math headers, display
resources, automatic driver catalogs, remaining named scopes/running content/
columns, full-book/public/manifests/managed-host/scaling and real author/human
acceptance remain open. table_repeated_frame_reflow remains in the private driver.
