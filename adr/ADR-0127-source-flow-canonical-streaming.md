# ADR-0127: Hash source-flow canonicals without retaining temporary strings

## Status

Implemented and locally verified under design 28 on 2026-10-06.
Overall design acceptance remains open.

## Decision

Core provides an incremental `Sha256` with private state, a fixed unfinished-block
buffer and consuming finalization. Complete update blocks are borrowed; padding
uses a fixed tail. The existing one-shot `sha256` delegates to the same
compression path, and a `fmt::Write` implementation accepts canonical bytes
without collecting them in a String. No workspace production unsafe code or new
dependencies are introduced. This utility grants no receipt/profile authority.

Legacy production flow and Book /2 share one source-flow canonical writer. It
preserves all existing field order, optional named-break behavior, JSON escaping,
hash spelling, styles, spans, text, number spelling and algorithm identities.
It streams directly into SHA-256 instead of retaining the entire canonical and
per-member formatted strings. Book /2 also streams its existing language and
five-field limits projections rather than retaining their canonical strings.
Full source-owner identity and collection/field revalidation remain enforced.

## Verification

Evidence is stored under
`workspace/target/vmb-design/20261006/source-flow-streaming/`.
A standalone client was built against the pre-change public libraries and again
after the change. All seventeen successful legacy/Book /2 projections and seven
typed rejections match, including named breaks, candidate page labels, languages,
empty footnotes and escaped multilingual text with 1,024 paragraphs. All 38
construction/verification allocation observations are retained. Successful
operations use fewer allocation calls and requested bytes. The largest Book /2
case's preparation requests 22,850,481 bytes before and 15,775,637 after; these
are cumulative allocation requests, not peak memory.

An independent Python hashlib oracle verifies 3,801 streamed, one-shot, writer
and original-input hashes, including thirteen chunk sizes and empty updates,
padding/block boundaries, UTF-8/control text and both original fonts plus the
preserved whole-book package. All measured hash operations allocate no heap.
Only the ignored standalone clients' standard allocator forwards use unsafe.
The first client fixture attempt retained outline entries after replacing their
source nodes and encoded a named page as a keyword; it was archived and corrected
before any production change. Existing legacy parser rejections of the large
fixture remain preserved; Book /2 accepts all three large cases.

Core 21 tests, math eight tests, legacy syntax 76 tests with six doc-tests and
Book /2 syntax 140 tests with twelve doc-tests passed. New tests cover every
padding-boundary input split and empty updates, known hash writer vectors,
five pre-change Book /2 fingerprints and all bounded canonical-sink capacities.
Final workspace checking, 70 layout tests with one doc-test and 294 Book /2 CLI
tests passed. The CLI filter runs 294 of the binary's 713 tests, with no failures
or ignored tests. Independent checks accepted 759 PDFs / 2,896 pages, 58 font
subsets and their embedded programs, running regions and uniform table pages.
All 759 PDFs and both saved VMB table outputs are byte-identical to stage 263.
The source-flow
visitor gained only a cfg(test) registration after public-client measurement;
its exact source correspondence and measured binary hashes are retained.

The correspondence report binds 166 frozen sources (all twelve changed files),
nine evidence clients/runners, three original inputs, twelve successful logs,
actual commands/exits/environment, all measured binaries/public results and all
PDF pairs. The three acceptance-document updates after verification are recorded
with separate hashes. No new shaping/pagination/font or public/legacy diagnostic
run or current-VMB audit is claimed. Timings are observations, not controlled
performance proof.

## Remaining scope

This removes the source-flow canonical/hash temporary storage. Collection still
retains rules, paragraphs, inlines, tables, generated text and navigation data.
Verification reconstructs the whole flow while the original remains held.
Collection/revalidation byte/AST/hash work, reservations before allocation,
failure retention and concurrently retained graphs still need command-budget
integration, together with admission, shaping/backend/font internals. Named
pages/columns, public Book /2, original whole-book, managed hosts, controlled
performance and author/visual acceptance remain open. Speech and SemanticRef
remain user-confirmed uncreated.
