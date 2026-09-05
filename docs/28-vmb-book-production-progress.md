# VMB book production implementation evidence

Status: In progress. This ledger does not narrow the scope of
[design 28](28-vmb-book-production-compatibility.md). No full-book success or
Harano support is claimed until the corresponding gates have evidence.

| Requirement | Current evidence / remaining work |
| --- | --- |
| ADR-0038 lexical exception | Implemented and covered by the 259-test run below |
| Safe-SVG 2 whitespace, multiple paths, curves, subpaths | Implemented; detailed-error / unchanged real-SVG tests passed in the 259-test run |
| SVG detailed reasons/spans/path/attribute/budget, JSON notes | In progress: command-level budget positions, clip replay, exhausted document budgets and paint/scalar/transform attributes now verified; remaining geometry/lexical detail cases below |
| Count/analyze/build internal mismatch diagnostic | Changed to receipt invariant / I9190; verification pending |
| Profile defaults and override precedence | Implemented; config tests and CLI negative boundaries passed |
| Original image/font count diagnostic pointer | Unit and both CLI runner boundary tests passed |
| Resolver cursor and finalized dense image lookup | Existing aggregate/order/admission regressions passed; mixed/5,000 performance evidence pending |
| Real VMB fixtures and provenance ledger | Unchanged chapter SVG plus 20 actual-engine conversions, original/derived hashes and font notices stored; original maximum-complexity book cases and full provenance runner still pending |
| 300–500 chapter and 5,000 placed distinct images / mixed aliases | Pending |
| 8,192 / 8,193 and explicit lower-limit CLI tests | 8,193 and explicit 1,025 rejection passed; 8,192 positive boundary pending |
| Detailed font diagnostics and TTC face list | Pending |
| CID CFF /2, FD-aware evaluator, subset / PDF integration | Pending |
| Vertical tables, cmap 14, IVS shaping/extraction | Pending |
| Contract 1.5 / production-book-2 / resource-set 3 and capabilities | Pending; publish atomically only after gates |
| VMB exporter geometry / metrics / semantics / source mapping | Geometry lowering and source projection builder implemented in VMB; RenderBook traversal, semantic speech and final package/sidecar encoding remain pending |
| VMB runner, explicit font/layout, environment isolation | Pending |
| Production with no native math | Empty native authorization implemented and regression passed; PDF body-font independence is still pending |
| Shared body/math flow and selected text placement | Syntax flow, admitted authored-text shaping and LTR body/SVG inline candidate bridge implemented; actual-engine negative-origin, mixed body/formula candidates, zero-width explicit breaks and shared line-local glyph/SVG projection verified; generated labels, common pagination and selected PDF text still pending |
| TrueType full book, one package / PDF | Pending |
| Unchanged Harano full book, one package / PDF | Pending |
| Independent visual / baseline / spacing / extraction / tag verification | Pending |
| Performance, determinism, negative/tamper tests, old-profile regression | Pending |
| macOS / explicitly managed Linux host evidence | Pending |

## Execution record

Implementation branch: `codex/vmb-book-production`. Product changes and completed
verification are recorded below. Focused checks do not imply full-book success.


Focused results observed in this implementation turn:

- `cargo test --manifest-path workspace/Cargo.toml -p typaxis-resource-admission safe_svg_2 --locked`: 13 passed before detailed-diagnostic additions. Covers terminal whitespace IR/work equivalence, V1 rejection, multiple paths/subpaths and 1,000-path M/L/C/Q/Z corpus. This result does not cover the subsequently added diagnostic code.
- `cargo test --manifest-path workspace/Cargo.toml -p typaxis-document-package --lib resource_count_limits --locked`: 1 passed. Confirms original array pointer and N+1 count, and root-shape precedence.
- Initial CLI config compile found a wrong new field name (`max_font_faces`); fixed to the actual `max_fonts`. Next compile found missing exported constants for existing R711x/R712x/R713x codes; added the constants. No success claim from these failed runs.


Verification using the isolated target directory:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-resource-admission -p typaxis-document-package -p typaxis-cli \
  --lib --bins --locked
```

Result: CLI **162 passed, 3 ignored**; DocumentPackage **44 passed**;
resource-admission **53 passed** (259 passed total, no failures).
The 3 ignored tests require independent PDF tools and are not counted as verified.
The run covers config precedence/profile isolation, default 8,193 rejection and
explicit 1,025 rejection in both machine runners, no successful PDF on those
failures, original pointers, precise SVG token/span notes with one R7100 prefix,
real chapter SVG bytes, frozen SVG1 IR, aggregate work and out-of-order admission.
It does not prove 5,000 placed distinct resources or full-book correctness.

The first combined run found three issues and they were fixed: the test input
encoder needed a larger test-only limit to construct an over-limit package;
coordinate-range classification needed to distinguish decimal lexical validity
from its range constraint; the old negative-corpus adapter needed to recognize
new detailed errors while still asserting the frozen broad category. A subsequent
run caught loss of the existing missing-metadata root pointer; root-member
validation now retains its precise location. The final command above passed.

The existing target directory caused rustc to wait in `readdir_r` /
`__getdirentries64` while discovering dependencies (sample evidence under
`/private/tmp/typaxis-rustc-76850.sample`). Only this task's three pending
verification groups were terminated, then the command above used a fresh target.
No existing target files or other task's process were removed.


Real-input probes:

- `implementation-chapter-02`: unchanged chapter `check-package` succeeded. The initial build invocation omitted the required uncompressed policy and was rejected before layout. This is not a PDF success. A subsequent attempt confirmed check rejects the build-only `--no-compress` flag.
- `implementation-chapter-04`: with one shared TOML (contract 1.4, `pdf_stream_compression = "none"`), unchanged chapter check succeeded; build failed with `L5100: semantic_container is not allowed in this owner`. No PDF was published. `observed.json` and check/build diagnostics in that directory retain the command/results. Follow-up code investigation below identifies the actual native-math gate; this input has no semantic-container nodes.
- `implementation-book-font-probe-01/observed-02.json`: diagnostic-only full-book copy with the font replaced by the chapter TrueType passes the image count budget but fails at `P1102: semantic source span ownership mismatch`. This adds an exporter/source-mapping investigation; no source-span validation was relaxed. The original snapshot was preserved. An initial invocation had duplicated a configured root and was corrected by using a separate working directory.

Known remaining SVG-detail work includes precise context for every transform/paint/non-path geometry failure, exact segment context on budget failures, context for a zero remaining document budget and clip-reference replay positions. The first detailed-error tests do not close those requirements. Complete positive resource-count boundaries, 5,000 placed distinct resources and performance observations also remain pending.


Local implementation checkpoint: `9e98ab0` on `codex/vmb-book-production`.
No branch push or publication has been performed. All verification processes
started in this turn have reached terminal states; there is no outstanding wait.
The companion VMB document was updated with the common compression config,
actual CLI option names and full-book source-mapping investigation.

## Follow-up: empty native math and actual source ownership

Neither supplied package contains a semantic_container. The chapter has 158
inline and 10 block precomposed formulas, and no native math. The build called
`StagingMathProfileView::new_for_production` unconditionally, whose shared
constructor rejected empty native math with the misleading InvalidNesting error.
The production constructor now accepts the empty set, while the closed math
slice still rejects it. The same sealed package/limits/session checks remain.

A first regression attempted both check and PDF build after removing the two
native math nodes from the combined fixture (retaining its inline/block SVG
formulas). Check passed; build failed with `I9190: production tagged-PDF native
math mismatch`. Inspection identified body text emission using the first native
math font and requiring MATH. It also found fixed 10pt text advances and PDF text
positions/font size calculated from record count, instead of selected text lines.
The complete correction is now required by design §14; none of these failures is
treated as a successful PDF gate. No dummy native formula or font was added.

The committed regression tests production authorization, its sealed recheck and
check-package, with zero native formulas; it explicitly tests that the closed
math slice still rejects the same input. It does not claim to test a successful
build. Focused command:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis machine_production_book_1 --locked
```

Result: **5 passed**, 0 failed, 0 ignored. The broader command also completed:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-syntax -p typaxis-cli --lib --bins --locked
```

Result: CLI **163 passed, 3 ignored**; syntax **62 passed** (225 passed, no
failures). The ignored independent-PDF tests remain unverified. All processes
started for this follow-up reached terminal states. No branch push, full-book
publication or Harano success is claimed.

Read-only inspection of the original full package identifies its first source
ownership violation: paragraph node 5 has span source 0, `0..0`, while child math
node 7 at `/document/blocks/2/children/1` has `0..1`. Later child node 9 has `1..2`
and text node 10 returns to `0..0`, violating sibling start ordering as well.
All source_tex mappings were separately scanned for exact identity length and
ownership in their math node; that scan found no mismatch. The error must not
be misreported as a missing TeX identity mapping. VMB companion design §14 now
specifies a complete generated source projection, original-source provenance,
parent/child ranges and negative tests, without relaxing Typaxis validation.

## VMB geometry/source implementation and actual-engine bridge

VMB implementation branch: `codex/typaxis-book-export`. Added
`vmb-core/internal/rendertypaxis/{math_vector,math_geometry,source_map}.go` and
tests. The package is not yet registered as a complete renderer. Geometry
lowering verifies the original result/hash, converts coordinates and placement
metrics with bounded signed integer arithmetic, and preserves individual path
paint operations, curves, fill rules and subpaths. Relative coordinates resolve
before rounding. Contour collinearity and exact Bezier signed-area checks detect
the tested collapse cases; these checks do not replace independent visual gates.

The projection builder allocates source-order node/text IDs and closes parent
spans after children. It keeps generated-source offsets separate from author
provenance, gives repeated TeX different occurrence spans, validates UTF-8 and
exact identity, and rejects overlapping text mappings and unclosed owners.
Its connection to complete RenderBook wire traversal and final publication is
still required; unit success is not a complete exporter.

VMB checkpoint: `4cd1b05af72f7757f7336956f1acfad488553185`. A third generation
from this final committed code also reproduced all 31 fixture files exactly.

`vmb-core/tools/typaxis-math-fixtures` generates five authored test expressions
through the actual VMB 2.0.0 engine, inline/block at two font sizes (20 conversion
cases, 10 original SVGs and 20 derived SVGs). It emits hashes, metrics, TeX,
semantic speech and complete engine artifact identities. Two generations of all
31 files were byte-identical. The corpus, font source records and notices are
stored in `samples/machine-package/staging/production-book-1/vmb-book/engine-v2/`.
These are not 20 occurrences extracted from the user's book and are not approved
PDF visual/accessibility goldens. The original chapter regression remains intact.

Completed focused verification:

```sh
# In vmb-core:
GOCACHE=/private/tmp/vmb-typaxis-go-test go test \
  ./internal/rendertypaxis/... ./tools/typaxis-math-fixtures -count=1

# In Typaxis:
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis \
  machine_book_actual_vmb_engine_svg_corpus_is_admitted --locked
```

The VMB command passed (11 top-level tests, including actual engine conversions
and adversarial subtests; the generator has no unit tests). The Rust command
ran **1 test and passed**, checking the exact 20-case matrix, source/derived
hashes and admission of every SVG through public check-package. It does not
assert successful placement of those added resources.

The broader Go command across `internal/render/...`, `internal/rendermath/...`
and the new package first found an incorrect new negative test: its original
quadratic contour already had zero signed area. The input was corrected to a
nonzero contour that rounds to a retraced curve; the exact-area test now passes.
A later run of all these packages except `cmd/mathcorpus` passed. The exclusion
is an unresolved pre-existing input/artifact consistency gate, described below,
not a successful full-Go-suite claim.

Actual check/build and independent observations are retained under
`workspace/target/vmb-design/20260905/implementation-engine-bridge-*`:

| Probe | Evidence |
| --- | --- |
| 01 | Adding a second source was rejected by both commands: P1110, exactly one source. No PDF. The designed single projection is required. |
| 02 | One source, 20 admitted SVGs: check succeeded; build rejected formula-only inline node 93 because its negative origin puts the viewport left of the line boundary. No PDF. The origin was not zeroed or clamped. |
| 03 | 10 inline formulas with preceding/following text plus 10 block formulas: check and build succeeded. The 20 added nodes all have manifest selected/display/PDF-use facts. Every physical metric, source TeX hash and ActualText hash matches the engine fixture record. |

Probe 03 used the existing combined package (including native math and its test
font) as the host. It does not close the no-native-math/body-font requirement.
MuPDF **1.28.2** read and rendered the PDF's **2 pages**; Poppler **26.08.0**
extracted each of the five formula alternatives **4 times**. The 20 resources
contain 14 distinct derived SVG hashes, including genuine inline/block aliases.
The observed reading order **does not match** source order: the current bridge
groups inline and block content separately. A rendered page was inspected and
confirms that standard text does not use the selected line positions. Keep
`extraction-comparison.json`, `conversion-observations.json`, the independent tool
log and page images as failure evidence, not expected output. The tool versions
are observations, not a newly approved pinned external-tool policy. Source-order,
baseline/spacing, tags, full mask comparison and managed Linux gates remain open.

### Existing corpus/migration consistency gate

The complete Go run failed `TestCorpusArtifactsAreCurrentAndStrictCutoverFindingsAreZero`:
the registered source corpus has 97,704 entries while current book input produces
97,719. A temporary full regeneration found 205 added, 190 removed and 12 changed
source records, in existing fractions-book topic files that this work did not
modify. The source-set hashes are:

- Recorded: `73a97f496190844dbffb4e85b6e267d4659ebbc08d9f9dbcf66a7d5f7d6f0950`.
- Current: `81b2d9efdcf327fea1386f3c3fb332b7fa1e26366a30405212d24f292dbd01af`.

After temporarily installing the six generated artifacts, a full mathcorpus
test verified their freshness but failed the separate historical M3-09 migration
record binding. That record retains the old source-set hash and meaning-preserving
approval evidence. It was not rewritten or relabeled as approving later author
changes. The six tracked artifacts were restored to their pre-task bytes; no
engine corpus or historical approval record is changed by this implementation.
Regenerated artifacts and the comparison are in
`/private/tmp/vmb-typaxis-corpus-observation`; a compact comparison is also retained
in probe 03. A source inventory/migration audit is required before claiming the
whole repository suite or publishing the full-book gate.

Current inventory additionally reports **7,739 speech-unresolved findings** for
the fractions book's `ja/profile.epub` context. This is a different inventory
from the original package's generic-alt placement count. Do not relabel it as
7,739 unique PDF formula occurrences, silently manufacture semantic speech, or
count parser/geometry acceptance as accessibility completion.

All processes started for this follow-up are terminal. No full-book PDF, Harano
support, 5,000-resource placement, new profile/capability publication or branch
push has been completed. The next product work remains the formal exporter and
the shared selected body/math flow, including negative-origin line starts.


## Follow-up: budget positions and attribute diagnostics

The previous goal turn updated the two requirement maps in the design documents.
This follow-up changes the SVG admission implementation, with the full design
scope unchanged.

`record_segment` now preserves the failing command or repeated operand group,
subpath/segment ordinal and resource byte range when the Count visitor rejects
an emitted segment. Shape-local limits and observed counts are translated first
to the current resource and then, at the existing resolver boundary, to the
whole document. This prevents both lost context and double-counted prior work.
The implicit viewport clip is charged at the root and does not invent a source
path index. Zero remaining V2 node budget reaches that root diagnostic; the
frozen V1 early rejection is preserved.

Analyze retains bounded source information for each clip reference. A replay
budget failure now identifies the actual `clip-path` attribute, group/path,
source preorder and byte range of the reference that exceeds the budget. Stored
segments and replay charges keep the same accepted-input counters and IR.

The shared scalar-coordinate readers preserve V1 failures and give V2 failures
the offending attribute/value. V2 paint, opacity, stroke, fill-rule, transform,
clip-reference syntax and clip ID helpers retain their feature category and
attribute position. Invalid numeric syntax, coordinate range and non-positive
geometry are distinguished where the scalar parser can prove the reason.
Allocation/session failures are not converted into malformed input errors.

New regression coverage includes:

- A second path exhausting the budget, repeated M operands, C and Z, exact
  full-charge success, and source span/token agreement.
- Zero remaining node/segment budget, synthetic root clip and the second clip
  replay after a group reference; clip path numbering includes definitions.
- A stable-read resolver admitting one alias before the next declaration fails,
  retaining document-total limit, observed, used-before-resource and progress.
- Sixteen paint/transform/scalar attribute failures, checking bounded canonical
  notes, byte range and line/element identity.
- Both public check/build runners on the same V1-then-V2 input, with identical
  resource pointer, context and budget notes for three limit cases. A package
  byte offset is not substituted for an SVG offset; no failed build emits PDF.

Completed local commands:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-resource-admission --lib --locked
# 57 passed, 0 failed.

cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis --locked
# 165 passed, 0 failed, 3 ignored (existing external-tool gates).
```

The 222 passing tests include frozen V1 IR/charge, existing negative corpora,
unchanged VMB SVG and production artifact regressions. They do not close the
full SVG diagnostic requirement: point-list token errors, relative-coordinate
addition failures, some transformed/derived extent failures, and pre-tag lexical
failures still need precise context/reasons. Full-book layout, formal exporter,
Harano, placed 5,000-resource and independent-host gates remain open. No full-book
PDF success is inferred from this diagnostic work.


## Follow-up: syntax-owned production text flow

The preceding goal turn made verified product progress on SVG diagnostics
(commit `50356cb`). This follow-up starts the production flow required by §14,
using the current worktree rather than treating the existing placeholder PDF
text as a valid layout.

Added `typaxis-syntax/src/production_flow.rs` and the syntax-owned
`prepare_production_text_flow` entry point. The flow borrows a verified package
and computed-language registry. It retains body-only paragraphs as well as
paragraphs containing SVG/native math, with explicit begin/end boundaries for
lists/items, tables/head/body rows/cells, captions, semantic containers, page
breaks, display math and footnote definitions. Definitions remain in a separate
footnote region; their paragraphs are not silently turned into body content.

Paragraph font families, size, line height and block properties come from the
existing checked cascade and inheritance rules. Semantic-container styles are
read from the package owner. Vector-figure caption style inheritance preserves
the current vector contract. No fixed 10pt character advance, 20pt line height,
first-native-math font or generated reference label is introduced by this flow.

Each inline site retains its source owner, exact source/TextSpan and effective
language. Text borrows the package's UTF-8 bytes; emphasis/strong/link boundaries
remain visible to the future shaping owner. SVG math, native math, references,
footnote references, anchors and soft/hard breaks are typed objects. In
particular the native TeX and reference target are not emitted as substitute
body text. Reference labels and generated markers still require their owning
resolution/placement stages.

The new internal identity is `typaxis.production-text-flow/1`. Constructors and
flow storage are private; downstream shaping must take the flow plus an index,
not trust a copied site. Verification rejects another package instance and
recomputes events, styles, text charge and fingerprint. Borrowed bytes avoid
copying each text span; occurrence bytes and traversed nodes are bounded by the
package limits. Recursive traversal relies on the already validated syntax
nesting bound. This does not change the existing semantic receipt identity or
publish a new contract/profile.

Four new regressions cover the real combined fixture's 27 paragraph owners,
interleaved native/block/SVG math, table/caption/footnote region boundaries,
14pt heading versus 12pt body and tall-cell cascade, inline container/atomic
ordering, exact Japanese/non-BMP UTF-8 borrowing, event/text/foreign-package
receipt tampering, no-native-math input, and a missing body font size that must
produce an owner-specific error rather than a guessed default.

This is a completed syntax prerequisite, not a completed body-font or PDF fix.
The public production build still uses the old fixed-metric helpers and native
math font for standard PDF text. The new flow must next be consumed by admitted
body-font shaping and unified line/page selection, then selected glyph/CID,
structure and navigation projection. It is deliberately not wired into a writer
that would claim to use its source order while continuing to paint at unrelated
placeholder positions. Full-book and all other design gates remain required.

Completed local verification for this flow prerequisite:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-syntax --lib --locked
# 66 passed, 0 failed (4 new flow tests).

cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis --locked
# 165 passed, 0 failed, 3 existing external-tool tests ignored.
```

All verification processes for this follow-up reached terminal states. These
231 passing tests establish the new syntax prerequisite and old-path regression;
they do not demonstrate that the PDF writer consumes the new flow.

## Follow-up: admitted authored body text shaping

Added `typaxis-shaping/src/production_text.rs` with the sealed
`shape_production_authored_text` entry point and internal algorithm
`typaxis.production-authored-text-shape/1`. This consumes the syntax flow once
at its boundary, resolves each paragraph's declared family and size against the
admitted ledger, and uses the existing linked harfrust backend with the site's
effective language. Legacy shaper/equation-number calls retain their previous
language setting. No MATH table, native-math authorization or first-declared-font
fallback is used to shape body text.

The result retains paragraph/site owners, logical source order, TextSpan-backed
glyph clusters, script and bidi levels, original glyph IDs, advances/offsets,
selected face/hash/index/size and signed scaled hhea metrics. The metrics use
the existing checked ties-to-even conversion. hhea metrics are explicitly not
glyph ink bounds or a clipping rectangle. The original flow continues to own
line-height, block styles, emphasis/link boundaries and atomic objects.

Paragraph itemization sees authored text plus U+FFFC for atomic math and
unresolved reference sites and U+2028 for hard break. The original /1 context
inserted a space for soft break; the explicit-break follow-up below corrects
that to empty context bytes in /2, matching docs/05 and docs/07.
These placeholders are context only and never returned as painted glyph runs.
Resolved reference labels can change paragraph context and require reshaping;
pending reference owner IDs are exposed explicitly. Generated markers,
math geometry, bidi line reordering and line/page selection remain the
next stages' work. A paragraph without authored text has no fabricated body font.

Context allocation is bounded by `max_shaping_context_bytes`, and each backend
request retains the pinned harfrust output preflight. Retained runs + glyphs +
clusters are also charged cumulatively against `max_fragments` across the whole
document. Grapheme boundary checks and ordered site/run intersections use moving
cursors, rather than rescanning the whole paragraph per site. The sealed,
non-deserializable result borrows the exact flow and admitted ledger; verification
compares those owners plus limits/epoch. Hierarchical digests bind all output,
flow/admission/limit hashes and linked-shaper identity without constructing one
document-sized glyph serialization. This is not yet the cache for final shaped
lines, and no new public schema/profile capability is published.

Five new CLI integration tests use actual contained host admission, syntax
validation and profile preflight. They cover all 27 paragraphs of the combined
fixture with separate normal TT/TTC body fonts, 14pt heading versus 12pt body,
pending references and atomic objects; a native-math-free CFF body with distinct
letter/space advances and exact source clusters; split grapheme, missing font,
uncovered non-BMP character, backend context ceiling; document-wide output
limits (two paragraphs accept 6 records and reject 5 at the second text owner);
and deterministic output/foreign-flow/epoch rejection. Positive CFF evidence is
for the existing name-keyed fixture, not Harano CID CFF or IVS support.

The first run exposed an additional old-fixture problem: the production combined
`body.ttf` and `collection.ttc` have ASCII in cmap format 4, while their preferred
Windows full-repertoire format 12 only contains added math mappings. The existing
scalar coverage helper can find ASCII in format 4; harfrust selects format 12 and
emits visible glyph 0 for `Basic document`. The new body path rejects this with
`MissingShapedGlyph` at owner 2 and TextSpan `(text_id=0, start=0, end=1)`. Glyph 0
is also rejected at zero advance: that alone cannot prove it has no visible ink.
Default-ignorable input is exempt from scalar coverage, but backend output that
uses glyph 0 to hide it still needs an explicit suppressed-cluster owner before
this path can accept it. A negative regression preserves the original conflicting fixture. Positive
tests copy the already checked-in basic-document-1 MATH-free TT/TTC fonts into an
isolated test job with their actual hashes; the frozen production fixture is not
rewritten or silently treated as a valid body-shaping oracle. Fixing its cmap
generator and separating native/body font usage remains required when publishing
the new production layout. This observation concerns Typaxis fixture fonts and
is not evidence of the same defect in VMB's Japanese fonts.

This is a completed shaping prerequisite, not a completed production PDF fix.
The public runner still uses the old fixed metrics and standard-text PDF painter.
Next, generated text/context resolution and shaped cluster selection must feed
one body/math line/page owner, then selected glyph/CID, links and structure must
be projected from it. Standalone text-only staging acceptance also remains
restricted by the old parser's requirement for a semantic/math/vector owner;
the native-math-free positive test therefore uses a semantic container. Japanese
positive shaping, GSUB/GPOS/locl-specific cases, common line selection, full-book
PDF, Harano and all original §10 gates remain required.

Completed verification:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-shaping --lib --locked
# 24 passed, 0 failed.

cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis --locked
# 170 passed, 0 failed, 3 existing external-tool tests ignored.
```

The CLI test-only JSON fixture editor uses the already locked serde_json 1.0.151;
the lock change only adds it to typaxis-cli's dependency list. No production JSON
decoder or dependency version was replaced. Both verification processes reached
terminal states; no full-book render/extract success is inferred from 194 tests.

## Follow-up: shaped body/SVG inline candidates (2026-09-06)

Added `typaxis-layout/src/production_inline.rs` and
`typaxis-linebreak/src/production_inline.rs`. The first joins exact shaped
clusters and resource-bound inline SVGs in the syntax flow's source order.
`prepare_production_inline_items` verifies the package/navigation, vector
bindings and shape's admission/limit/epoch owners once at the stage boundary.
The immutable result borrows all three owners. Source cluster indices remain
paired with their glyph runs; the Unicode scalar projection is exclusively for
line-break classification and is not a replacement text painter. A cluster's
actual glyph advances are summed and placed on its final scalar unit; all
internal boundaries are prohibited. Scalars do not get guessed advances.

The new `typaxis.production-inline-break/1` kernel retains the old UAX #14 and
Japanese boundary rules and producer spacing, while supporting body-only
paragraphs and complete shaped cluster ranges. Cluster gaps, overlap, vector
crossing and crossing a mandatory break are rejected. The old atomic-vector
API still requires a vector and still rejects negative-origin empty-line
overhang; its frozen outputs are unchanged. The new itemization and selection
have distinct fingerprints, including Japanese mode and cluster partition.

Candidate fitting uses `left=min(0, visual_left)` and
`right=max(logical_advance, visual_right)`, then tests `right-left`. Each selected
candidate records `origin_shift=-left` separately. Glyph and SVG placement must
both apply it; no resource metric, origin, spacing, advance or viewport is
overwritten. The kernel extends each candidate in constant time with monotonic
advance/extents and stops after irreversible overflow, avoiding the old repeated
prefix measurement. Only selected lines collect vector occurrences and vertical
metrics. A shared `ProductionLineBreakBudget` charges candidate-unit visits and
selected lines across calls, with distinct `CandidateLimit` and `SelectionLimit`
errors. The future document selector must retain that budget, choose its finite
effective ceilings, and charge all paragraph calls; no public configuration
field or capability is claimed for this internal owner yet.

Four new CLI tests traverse contained resource admission, syntax, profile,
vector binding, body shaping and the new preparation/kernel. They use the actual
checked-in VMB engine 2.0.0 `fraction-inline-720896.svg`, with its fixture-index
hash, provenance, TeX identity mapping, speech and unchanged metrics. The
formula-only case accepts exactly 1,556,093 raw units and rejects one raw unit
less. Its original origin remains -45,056 and advance 1,465,981; line origin shift
is +45,056. The mixed `A <formula>B` case has three body clusters, one atomic
formula, formula pen 707,789, logical advance 2,645,629 and line height 958,936,
all from admitted metrics. A MATH-free CFF body-only paragraph enters the same
kernel. At this /1 milestone, a paragraph with authored text followed by an unresolved
soft break returned the break's owner. The /2 follow-up below connects it as a
zero-width opportunity and replaces that pending-error test.

The CLI admission test helper was corrected to pass the precomposed profile
receipt fingerprint to resource admission, matching the public runner. Its old
authorization fingerprint happened to be sufficient for standalone shaping but
correctly failed when the SVG binding stage rechecked the attestation. No
production authorization check was weakened to make this test pass.

Three additional kernel regressions cover negative-origin compensation while
the old API continues to reject it; body-only Japanese scalar candidates with
one versus two shaped clusters; exact candidate/line budgets; and invalid
cluster partitions/mandatory breaks. The Japanese kernel case is not evidence
of Japanese font shaping. The suite covers candidate geometry, not rasterized
ink or PDF placement.

Remaining work is explicit. This bridge currently returns owner-specific pending
errors for references, footnote markers, native math and nonzero bidi levels.
Soft/hard breaks were pending in /1 and are connected by the /2 follow-up below.
It does not erase those sites or treat this subset as the final production domain. Empty/anchor-only paragraph records are retained for
the containing flow. The whole-book selector still needs generated text, break-only paragraph
profile/structure integration, line-boundary reshaping, bidi line ordering, text ink extents,
line-edge whitespace/justification, block math/figures, table/caption/footnote
placement and common page selection. PDF glyph/CID/ToUnicode, structure and
navigation must then consume those same selected positions. The public
`build-package` runner is not yet wired to this bridge. The earlier failing
full PDF probes therefore remain failures, and §10's complete scope stays open.

Completed local verification after the final changes:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-linebreak -p typaxis-layout -p typaxis-cli --lib --bin typaxis --locked
# CLI: 174 passed, 0 failed, 3 existing external-tool tests ignored.
# Layout: 65 passed, 0 failed.
# Linebreak: 40 passed, 0 failed (3 new kernel tests).
```

The process exited successfully; 279 tests passed. The local log is
`/private/tmp/typaxis-production-inline-verification.log`. The two new actual-VMB
candidate tests do not invoke the public PDF writer or independent renderer,
and are not substituted for the required check/build/render/extract book gates.


## Follow-up: zero-width explicit paragraph breaks (2026-09-06)

The production layout bridge now emits typed `ProductionExplicitBreak` units
with the syntax node's owner and source span. SoftBreak is an Allowed boundary;
HardBreak is Mandatory. Neither creates a space, glyph, TextSpan or SVG.
`shape_production_authored_text` no longer inserts a soft-break space into its
context. HardBreak retains U+2028 solely in shaping context and uses a typed BK
unit in the Unicode line-break classifier. The shape, inline preparation and
production break algorithms are bumped to /2; the frozen atomic-vector kernel
keeps its existing algorithm and behavior.

Each explicit break owns the boundary after its zero-width item. The boundary
before the first break is prohibited, avoiding duplicate Unicode opportunities.
A terminal soft break is promoted to Mandatory; a terminal hard break does not
create a further blank line. Consecutive hard breaks preserve empty lines with
the paragraph's computed line-height. Cluster ranges cannot cross a break and
continue to address the expanded unit sequence, including controls. Duplicate
break owners, paragraph-owner reuse and vector/break node collisions are rejected.
Preparation charges each control to the existing document fragment ceiling, and
the shared candidate/selection budget also charges break-only lines.

Producer spacing crosses an unselected soft break only when the adjacent painted
content remains on the same selected line. At a selected boundary, the previous
SVG's after-spacing and next SVG's before-spacing are removed. Candidate fitting
and selected measurement use the same previous-content lookup; vector occurrences
retain original metrics and report the applied edge spacing. Selected measurement
has its own metrics record; it does not fabricate a successful fit flag. The
existing compensated visual-extent check still validates each selected line.

CLI regressions exercise actual admitted body shaping, retained break owners,
wide/narrow optional wrapping, leading/consecutive/terminal hard breaks, terminal
soft breaks. Kernel regressions cover break-only lines and additionally cover
SVG-before/after spacing across soft/hard breaks, owner collisions, cluster
crossing and typed Unicode BK boundaries. This is stage-level evidence; generated
labels, bidi final-line ordering, boundary reshaping, text ink, whitespace and
justification, common pagination and selected PDF painting remain pending.

Completed local verification:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-linebreak -p typaxis-layout -p typaxis-shaping -p typaxis-cli \
  --lib --bin typaxis --locked
# CLI: 175 passed, 0 failed, 3 existing external-tool tests ignored.
# Layout: 65 passed, 0 failed.
# Shaping: 24 passed, 0 failed.
# Linebreak: 44 passed, 0 failed.
```

308 tests passed. Log: `/private/tmp/typaxis-production-break-verification.log`.
The public PDF runner is still not connected to these new stages. No full-book
check/build/render/extract, Harano support or completed exporter claim follows
from these tests; the remaining scope in §10 is unchanged.

The break-only CLI probe remains outside the current tagged profile: a document
whose semantic container contains only breaks fails BaseProfile; adding a real
body paragraph passes that stage but the break-only paragraph is rejected as
UnsupportedSemantic. The committed positive break-only tests therefore target
the line kernel, not CLI admission. The production profile/structure extension
must explicitly handle these unpainted paragraph owners when connecting common
pagination and PDF semantics. No dummy text or fabricated semantic content is
introduced to bypass those guards.


## Follow-up: shared line-local body glyph and SVG positions (2026-09-06)

Added `typaxis-layout/src/production_selected_inline.rs` and
`layout_production_inline_lines`. This selects every prepared paragraph once in
source order, requires an equally sized list of available paragraph widths and
retains empty/structural records. It returns a sealed result borrowing the exact
prepared owner. Its `verify` rejects another preparation, including an independently
admitted copy of identical input. Selected widths, preparation identity, paragraph
owners and kernel selection fingerprints are bound to the new
`typaxis.production-inline-line-layout/1` digest.

The production kernel is now `typaxis.production-inline-break/3`. Selected-line
measurement retains every logical unit's pen after applicable same-line spacing,
including zero-width controls and intra-cluster scalars. These pens are encoded
in the selected line's canonical identity. The layout projection reads the start
pen of the original shaped cluster, then uses the original glyph advances and
X/Y offsets. It verifies the cluster's final pen against selected unit metrics.
A cluster retains its original run, source TextSpan and exact UTF-8 slice; each
glyph refers to the admitted shape's original record. There is no per-scalar
cmap lookup, reconstructed glyph advance or guessed text string.

Line-local coordinates use a top-left, Y-down system. The baseline is selected
leading-before plus maximum content ascent. Glyph X is shifted pen plus the
shaper's X offset; glyph Y is baseline minus the shaper's Y offset. SVG geometry
uses the existing bound `select_inline_geometry` with the same shifted pen and
baseline. A negative SVG origin shifts the following body glyphs too. Output
items preserve text-cluster/vector/break source order; breaks carry no glyph or
extraction string. Markup and anchors remain owned by the borrowed syntax flow.
The paragraph retains its actual body font, and formula-only paragraphs retain
None instead of borrowing a native math font.

One candidate budget spans all paragraph calls. Retained paragraph, kernel line,
unit-pen, kernel occurrence, projected line, cluster, glyph, vector and control
records share the preparation's document `max_fragments` ceiling. Each is charged
before its allocation. The shared kernel line allowance is tightened to the
remaining downstream record budget before selection; it never restores consumed
visits/lines. The candidate ceiling remains an explicit internal argument for
the future common flow owner, not an already published CLI/capability setting.

Six CLI regressions traverse real resource admission, syntax, body shaping,
inline preparation, selection and line-local projection:

- Actual VMB fraction plus `A B`: glyph pens 0, 471,859 and 2,173,770; formula
  pen 707,789; common baseline 665,170; formula viewport left 662,733 and top 0.
  The cluster text reconstructs only the authored `A B`, with original source
  buffer IDs and actual glyph references.
- The negative-origin formula followed by `B` shifts both by 45,056 raw. Formula
  viewport left is 0, its pen is 45,056, and B's glyph X is 1,511,037.
- A producer after-spacing of 200 across an unselected soft break moves B's X to
  1,511,237. Selecting that break removes spacing and starts B at X=0 on line 2.
- Name-keyed CFF body uses its declared face and actual glyph IDs [1,3,2], while
  a formula-only paragraph needs no body font.
- Soft/hard breaks split exact source clusters without duplicate glyph/text and
  retain the control's source owner.
- Missing paragraph widths fail; two body paragraphs share the exact four-visit
  candidate budget and the 18-record placement budget. A three-visit budget
  fails on paragraph 2; a 17-record ceiling fails on its second glyph's owner.
  Repeated selection is deterministic, width changes change identity and another
  prepared owner cannot be substituted.

Completed verification:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-linebreak -p typaxis-layout -p typaxis-cli --lib --bin typaxis --locked
# CLI: 181 passed, 0 failed, 3 existing external-tool tests ignored.
# Layout: 65 passed, 0 failed.
# Linebreak: 44 passed, 0 failed.
```

290 tests passed; log `/private/tmp/typaxis-production-line-projection-verification.log`.
These are line-local placements, not PDF paint authorization or paginated output.
Page/frame selection, native math/generated-label joining, final-line reshape and
bidi, text ink extents, justification, block/table/footnote layout, PDF font usage,
structure/navigation projection and the whole-book gates remain open. Nonzero
GPOS offsets, ligatures, Japanese fonts and independent rendered baselines still
need their own positive evidence. Public PDF writer behavior is unchanged, and
no successful full-book PDF is inferred from this stage.
