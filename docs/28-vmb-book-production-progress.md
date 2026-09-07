# VMB book production implementation evidence

Status: In progress. This ledger does not narrow the scope of
[design 28](28-vmb-book-production-compatibility.md). No full-book success or
Harano support is claimed until the corresponding gates have evidence.

| Requirement | Current evidence / remaining work |
| --- | --- |
| ADR-0038 lexical exception | Implemented and covered by the 259-test run below |
| Safe-SVG 2 whitespace, multiple paths, curves, subpaths | Implemented; detailed-error / unchanged real-SVG tests passed in the 259-test run |
| SVG detailed reasons/spans/path/attribute/budget, JSON notes | Command/points budget positions, clip replay, exhausted document budgets, paint/scalar/transform attributes, lexical preflight and incomplete start-tag positions verified; see 2026-09-07 checkpoint for scope |
| Count/analyze/build internal mismatch diagnostic | Changed to receipt invariant / I9190; verification pending |
| Profile defaults and override precedence | Implemented; config tests and CLI negative boundaries passed |
| Original image/font count diagnostic pointer | Unit and both CLI runner boundary tests passed |
| Resolver cursor and finalized dense image lookup | Existing aggregate/order/admission regressions passed; mixed/5,000 performance evidence pending |
| Real VMB fixtures and provenance ledger | Unchanged chapter SVG plus 20 actual-engine conversions, original/derived hashes and font notices stored; original maximum-complexity book cases and full provenance runner still pending |
| 300–500 chapter and 5,000 placed distinct images / mixed aliases | 5,000 actual-SVG aliases and 5,000 synthetic distinct-paint Forms pass selected placement/structure/object tests; required engine-generated distinct formulas, mixed PNG and public check/build gates remain pending |
| 8,192 / 8,193 and explicit lower-limit CLI tests | Both public check/build positive 8,192 and explicit 1,024 boundaries, and negative 8,193 / 1,025 boundaries passed; see 2026-09-07 record |
| Detailed font diagnostics and TTC face list | Admission/table/permission and bounded container/face notes connected to both public runners; unchanged Harano negative gate passed below. Detailed selected-glyph/charstring/subset failures and all TrueType metadata stages remain pending. |
| CID CFF /2, FD-aware evaluator, subset / PDF integration | Pending |
| Vertical tables, cmap 14, IVS shaping/extraction | Pending |
| Contract 1.5 / production-book-2 / resource-set 3 and capabilities | Pending; publish atomically only after gates |
| VMB exporter geometry / metrics / semantics / source mapping | Geometry lowering, source projection and production math-adapter→per-occurrence wire/resource/semantic binding implemented in VMB; a real prepared-example public check gate passed below. Full RenderBook traversal, raster integration and final package/sidecar publication remain pending |
| VMB runner, explicit font/layout, environment isolation | Pending |
| Production with no native math | Empty native authorization implemented and regression passed; PDF body-font independence is still pending |
| Shared body/math flow and selected text placement | Authored shaping and LTR body/SVG line placement, measured paragraph/block/caption/raster/list placement, shared body fonts, Forms and selected PDF contributions verified below. Page-end candidate costs are now connected to the internal body cursor; final reshaping/bidi, generated references, remaining subflows and public convergence/terminal/paint/manifest connection remain pending. |
| Measured body page-break candidates | Internal policy /1 enumerates all feasible non-keep boundaries, applies widow/orphan/heading/unused-space costs, retains candidates in pagination /4 fingerprints and rejects max+1 before evaluation. Generic trace/budget receipts and public runner integration remain pending. |
| Selected production structure / MCID page contributions | Source registry binding, page-local MCIDs, per-occurrence Formula ActualText and cumulative budgets verified below, including 5,000 aliases; final structure objects/public PDF connection pending |
| Selected production PDF object contributions | Frozen body fonts, shared Forms and structure objects verified below, including 5,000 aliases; diagnostic page/catalog/xref assembly and selected internal/URI links, destinations and outlines connected; public terminal/manifest closure pending |
| Selected production lists | Syntax-generated markers, admitted-font shaping, real nested item frames, one label on the first actual fragment, marker height, Lbl/LBody and diagnostic PDF now connected; 11 focused tests and 4 independent probes pass below. Numbered math, final generated-store convergence, public/full-book integration remain pending. |
| Selected ordinary raster figures | Explicit width/pixel aspect, shared PNG/JPEG Image payloads, alpha masks, real captions/keep and per-occurrence Figure/Alt connected to diagnostic PDF; three independent raster probes pass. Public/full-book integration remains pending. |
| Selected production navigation positions | Inline source-gap anchors and logical bounds now feed selected destinations, per-line internal/URI annotations, outline topology and OBJR/ParentTree objects; three independent navigation PDF probes pass. Anchor-only paragraph and public terminal/manifest closure remain pending; see the checkpoint below. |
| TrueType full book, one package / PDF | Pending |
| Unchanged Harano full book, one package / PDF | Pending |
| Independent visual / baseline / spacing / extraction / tag verification | Eight diagnostic PDF probes now pass structure/nonpainting and exact extraction, including the previously failing explicit post-formula space; full SVG/reference and full-book gates pending |
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


## Follow-up: one cursor for body lines, block SVGs and captions (2026-09-06)

Added `typaxis-pagination/src/production_body.rs` with
`paginate_production_body` and internal algorithm
`typaxis.production-body-pagination/1`. It consumes the preceding sealed line
layout and existing resource-bound block preparation, checking package, profile,
limits, admission, binding set and epoch identities before building work items.
The returned immutable selection borrows those exact owners; its verify rejects
another line or block owner even when independently produced from identical input.
Paragraph and block indices stay connected to the original line glyph/SVG and
block geometry records. The fragment array preserves source order across types.

The syntax flow is now `typaxis.production-text-flow/2`. Paragraphs retain their
computed page name, which the original inheritance-only record dropped. The
ordinary closed cascade is reused. Named page selection is still pending in
this cursor and returns an owner-specific error; the setting is not silently
ignored. Source-flow access to semantic container styles retains package ownership.

The cursor uses paragraph start/end indent and start/center/end alignment,
selected line heights, producer-sized block content heights, and before/after
spacing. Spacing is summed at same-page boundaries and discarded at a fresh
page edge. Container vertical spacing and final keep pass to the first/last
content item; nonzero container indents remain a pending owned region. Required
line width determines aligned bounds. A zero-width hard-break-only line keeps
its real height and unshifted inner frame, so end alignment cannot move a blank
full-width rectangle past the body edge.

`keep_with_next` uses actual following line/block heights and boundary spacing.
Keep chains are premeasured in a bounded suffix array rather than repeatedly
rescanning prefixes. A chain moves together when remaining space is insufficient;
a chain larger than the body fails with its owner. `keep_caption=true` connects
the SVG block through all measured caption lines; false permits caption lines
to continue onto the next page. No fixed 20pt caption or successor estimate is
used. A keep chain crossing an explicit page break is rejected as a conflict.

Every explicit page-break node produces the next page, including leading,
consecutive and trailing breaks. The initial page is retained even for an empty
flow. Page limits are charged before allocation. Line-layout records, prepared
blocks, ordered work items, keep extents, selected pages/fragments and explicit
break records share the document fragment ceiling. Geometry arithmetic and
fragment ordinals use checked operations before publication/allocation.

Eight new CLI tests traverse the actual VMB engine 2.0.0 inline and block fraction
fixtures with their original metrics, hashes and identity source mappings. The
block SVG is a distinct declared resource; its repeated TeX gets a new source
projection range and text buffer, preserving parent ownership and ordering.
The initial test fixture used an already occupied image ID and renumbered the
Document root from 1; both fixture errors were corrected without relaxing parser
checks. Named page values use the wire `string` kind, lowered to StyleValue::Text.

Observed positive geometry on a 3,000,000-raw-height body:

- Body/inline paragraph owner 2: page 0, top 655,360, baseline 1,320,530.
- Block fraction owner 6: page 0, top 1,745,368, viewport left 1,769,183,
  height 1,552,815 and baseline 2,758,589. The actual same-page gap is 131,072.
- Following body owner 7: page 1, top 655,360, with page-edge space removed.
- With block keep-with-next enabled, owners 6 and 7 both move to page 1; their
  measured group consumes 2,601,391 raw. A 2,500,000-high body rejects that group.
- Vector/caption keep true yields fragment pages [0,1,1,2]; false yields
  [0,0,1,1], using an actual 917,504-raw caption line.
- Four explicit breaks produce five pages with fragment counts [0,2,0,1,0].
  max_pages=5 passes and 4 rejects. Shared max_fragments=33 passes and 32 rejects
  at the following body's owner. Repeated selection has the same fingerprint.

Additional tests reject list-flow flattening, retain and diagnose the named
paragraph page, and keep an end-aligned empty hard-break line within the body.
No dictionary/collection of different content types gets independently paginated
and subsequently overlaid.

Completed local verification:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-syntax -p typaxis-shaping -p typaxis-linebreak -p typaxis-layout \
  -p typaxis-pagination -p typaxis-cli --lib --bin typaxis --locked
# CLI: 189 passed, 0 failed, 3 existing external-tool tests ignored.
# Syntax: 66 passed, 0 failed.
# Shaping: 24 passed, 0 failed.
# Linebreak: 44 passed, 0 failed.
# Layout: 65 passed, 0 failed.
# Pagination: 85 passed, 0 failed.
```

473 tests passed; log `/private/tmp/typaxis-production-body-verification.log`.
This is an internal forward placement stage with measured keep groups. General
bounded lookback/cost selection, widow/orphan rules, reference convergence,
list markers, raster figures, native math, tables, footnotes, named pages and
container horizontal frames still require integration. The current function
rejects unconnected regions instead of flattening or omitting them. Math terminal
receipts, common PDF font/paint usage, tags/navigation and the public build runner
are not yet wired to these page positions. The full-book PDF and all §10 gates
remain open; this evidence is not an independent PDF render/extract result.


## Follow-up: selected page-space display, shared body CIDs and PDF text contributions (2026-09-06)

Added `typaxis-display-list/src/production_body.rs`,
`typaxis-resources/src/production_body.rs` and
`typaxis-pdf/src/production_body_text.rs`. The display borrows the exact common
selected pagination and admitted ledger; it projects retained original glyphs
and bound SVGs in source order. Each glyph receives the selected fragment's
page origin plus its selected line-local position. Text keeps its actual face,
font size, source cluster and UTF-8. Inline and block vectors retain common/math
bindings, page indices and actual viewports. Equation numbers are explicitly
pending at this new projection boundary and are never silently dropped.

The production font bridge checks display/ledger/limits ownership before
creating any usages. Its TrueType/TTC finalizer shares one CID per requested
original glyph in deterministic GID order, while the old equation-number recipe
continues assigning its frozen source-cluster CID sequence. CFF already shares
glyph CIDs. Composite closure remains in the actual TrueType subset. Ambiguous
single-glyph Unicode mappings, ligatures and complex clusters preserve exact
text using cluster ActualText when CID Unicode concatenation does not match.
The mapping-level negative/ambiguous tests intentionally supply artificial
usage lists; they are not evidence of authored shaping for those sequences.
Font/cluster lookup is indexed or binary searched, not an occurrence-by-cluster
linear scan. Record budgets continue from selected layout/display and include
conservative temporary usage/glyph/CID copies; copied text, subset and output
bytes retain finite limits. Each font also checks the effective subset ceiling.

`encode_production_body_text` consumes only the sealed production font plan.
It writes the actual selected font size and glyph coordinates with shared CIDs,
resets text spacing/scaling/rise, restores graphics state and emits cluster
ActualText once when required. `/PB{id}` identifies the frozen font that the
future final page dictionary must bind. Text contributions carry their draw and
page index; vector draws do not create text or dummy glyphs. These contributions
are not a complete PDF and do not invent MCIDs or authorize untagged export.

Verification:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-display-list -p typaxis-resources -p typaxis-pdf \
  -p typaxis-layout -p typaxis-pagination -p typaxis-cli \
  --lib --bin typaxis --locked
```

Observed **504 passed**, no failures: CLI 194 passed / 3 existing external-tool
ignored; display-list 56; resources 28; PDF 76; layout 65; pagination 85.
Log: `/private/tmp/typaxis-production-font-verification.log` (local execution
evidence, not a permanent release artifact).

New integration assertions cover:

- Actual VMB inline/block fractions and surrounding 12pt MATH-free body text
  over two pages. The first body glyph is at x=720,896 / y=1,320,530 raw;
  encoded text matrix is `1 0 0 -1 11 20.149688720703125 Tm`.
- TT, TTC and name-keyed CFF repeat the same glyph at distinct source spans
  under a one-CID limit; each source occurrence remains present.
- 2,048 actual prepared/shaped/paginated paragraphs of 32 `A` glyphs produce
  65,536 selected text draws and 65,536 PDF `Tj` contributions with one font CID,
  maintaining page/source order. This is a body text scaling test, not the
  distinct 5,000-image or real-book gate.
- Formula-only input creates no body font/paint. Another display or font plan,
  even with matching content, cannot replace the exact owner. Another admitted
  ledger or changed effective limits is rejected.
- Exact and one-less cumulative font/paint record ceilings; bounded output
  failure. Existing vector, font subset, resource, layout and frozen PDF tests
  continue passing.

Public `write_production_tagged_pdf_v2` still uses the previous final writer.
Required next work includes integrating these contributions into one final
object/structure/navigation plan, vector forms at the same selected positions,
equation numbers and other body subflows, and eliminating the fake old standard
text paint path. Independent rendered/extracted PDFs, semantic tags/links,
Harano CID CFF, the formal VMB exporter, 5,000 placed distinct images and the
whole-book gates remain incomplete. This step does not change those conditions.


## Follow-up: shared SVG Forms and source-ordered body page content (2026-09-06)

`ProductionBodyVectorDraw` now retains the content key, bound scale/currentColor,
page placement matrix and an occurrence fingerprint derived from the selected
owner. The existing admitted-resource closure checks determine the key; the PDF
backend never reopens or rewrites SVG bytes. Resource declarations remain
borrowed from the exact syntax package behind the selected display.

Added `typaxis-resources/src/production_vectors.rs` and
`typaxis-pdf/src/production_body_pages.rs`. Vector finalization starts from the
sealed body font plan and charges the combined text/vector records before
allocating Form joins and PDF page/usage collections. It reuses the admitted
candidate registry and content-key-sorted Form planning. Every selected alias
has its own usage count, including zero-use aliases in the selected candidate;
Form streams remain free of Alt/ActualText/MCID. Per-occurrence common/math
bindings remain in the display. The existing `/2` Form recipe is reused without
pretending that the new selected display is an old staging display receipt.

`build_production_body_page_content` merges actual body text and SVG `Do`
contributions in the selected draw order. The content has one root Y flip per
page, including explicit blank pages. Draw indices and byte ranges let the next
structure owner insert marked content around the actual paint, rather than
reconstructing its location. Text-only pages have no vector resource binding;
formula-only content requires no body font or dummy text. Spool accounting
continues from the font plan through text, vector streams and merged content;
the text encoder now reserves only the remaining spool allowance after fonts.
Page-content output bytes and cumulative record limits are checked at exact and
one-less boundaries. The final PDF object graph still needs its global object
and output budget closure.

The reusable vector PDF writer groups inputs by page once and binary-searches
aliases, replacing page-by-all-usages scans. Existing contribution fingerprints,
Form isolation tests and frozen PDF regressions remain unchanged.

### Performance finding from the 5,000-alias execution

An initial live CPU sample found `validate_languages_v2` calling whole-package
`checked_wire` from each vector-language verification. Thus 5,000 vectors caused
repeated validation/hashing of all 5,000 metric receipts. The old run was
explicitly terminated after identifying and fixing this quadratic operation;
it is not counted as passed. Evidence:
`/private/tmp/typaxis-5000-alias-sample.txt`.

Added syntax-owned `PrecomposedVectorVerification`, constructible only after
full package verification and holding an immutable borrow. Batch language,
structure, resource-profile, layout and equation-number shaping paths verify
individual metrics/language/style against this scope. Foreign-owner/session
metrics, tampered language and package-wide corruption still fail; the old
standalone API retains full validation. A scope cannot be created from a claimed
hash or arbitrary copied receipt, and the borrow prevents package mutation while
it remains live. Style ownership lookup uses the requested node directly.
This changes verification work sharing, not public receipt content or admission
charges.

A later sample found the test setup copying the same staged SVG once per alias.
The helper now copies each URI once, while the actual resolver continues reading,
hashing and admitting every declaration. That old setup run was also explicitly
terminated for the confirmed fix. Evidence:
`/private/tmp/typaxis-5000-alias-scoped-sample.txt`.
The subsequent current-run sample showed ordinary batch language/profile work;
no repeated whole-package check per vector remained on that sampled path.
These samples are local diagnostic evidence, not a formal release benchmark.

### Verification

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-syntax -p typaxis-shaping -p typaxis-machine-profile \
  -p typaxis-layout -p typaxis-pagination -p typaxis-display-list \
  -p typaxis-resources -p typaxis-pdf -p typaxis-cli \
  --lib --bin typaxis --locked
```

Observed **648 passed**, no failures: CLI 198 passed / 3 existing external-tool
ignored; display-list 56; layout 65; machine-profile 50; pagination 85; PDF 76;
resources 28; shaping 24; syntax 66. The CLI suite, including the new 5,000-alias
case, took 226.11 seconds in this debug run. This is suite elapsed time, not an
isolated check/build benchmark or a promised performance target.
Log: `/private/tmp/typaxis-production-page-verification.log`.

New assertions cover source-order text/real inline fraction/text/block fraction,
exact selected matrices, real curve commands in Forms, mixed text/vector pages,
blank-page preservation, shared aliases with distinct ActualText retained on the
source bindings, unused aliases, deterministic contributions, foreign font-plan
owners and cumulative record/output boundaries. The initial boundary-test failure
was caused by using `u64::MAX`, which is not JSON-safe; the test was corrected to
the valid default output ceiling before the successful full run.

The large case has **5,000 image declarations, 5,000 admitted aliases, 5,000
selected inline occurrences and 5,000 page `Do`s sharing one Form**. It uses the
unchanged actual engine-v2 fraction SVG with separate generated source spans and
text buffers per occurrence. The test explicitly sets max_images=8,192. It does
not prove the default-policy boundary, 5,000 distinct SVGs, mixed PNG, 8,000-plus
occurrences, public `check-package`/`build-package` or an independently rendered
final PDF. Those gates remain open.

Next required connection is the final PDF object/structure/navigation owner:
selected font dictionaries and Form objects, occurrence-local Formula/Figure
semantics, marked content and extraction anchors, destinations/links and public
writer closure must all consume these selected pages. Equation numbers and the
remaining subflows still need the common display bridge. Harano CID CFF, the
formal VMB exporter, distinct-image and whole-book gates remain incomplete.

## Selected production structure and marked page content (2026-09-06)

`typaxis-display-list/src/production_structure.rs` now derives structure ownership
from the actual `ProductionBodyDisplay`. It uses the package/navigation borrowed
by that display's source flow, verifies the navigation → vector profile and
accessibility → navigation profile chain, and builds the existing V2 source
structure registry. A source-node index joins selected text/vector draws to
registry nodes without a linear lookup for each paint. Required registry paints
must all be present. Vector kind, metrics, alternative, resolved math ActualText
and an explicit binding language must agree.

Contiguous text clusters from one source owner and selected line share a group.
A subsequent line/page gets another source-fragment ordinal; its page gets a
fresh dense MCID sequence. Each vector occurrence gets its own group and dense
usage ID. The plan retains node → group and page → group indexes for the final
MCR and ParentTree owner. Empty pages have empty indexes, without dummy paint.
The groups are private-field records borrowed from the exact display, and the
final merged contribution rejects a foreign display/content/ledger even when
its deterministic fingerprints match. Semantic/profile authorizations remain
deterministic values: reproducing the same inputs is allowed by their existing
contract; different semantic content is rejected.

`typaxis-pdf/src/production_body_marked.rs` encloses the retained per-draw bytes
with role/MCID/Lang marked content, keeping the common page root transform only
once. Vector ActualText comes from its occurrence's registry node. It does not
repeat a standard text node's full string on every cluster, line or page: body
text retains the frozen cluster ToUnicode/ActualText encoder. Shared SVG Forms
remain free of MCID/Alt/ActualText. Registry strings and marked-stream copies
participate in the cumulative spool budget; the font/vector and structure
branches combine record charges without restarting their shared display budget.

Verification:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-syntax -p typaxis-layout-contract -p typaxis-display-list \
  -p typaxis-pdf -p typaxis-cli --lib --bin typaxis --locked -- \
  --skip production_body_page_content_places_5000_real_svg_aliases_with_one_shared_form
```

Result: **405 passed**, no failures (CLI 200, display-list 56,
layout-contract 7, PDF 76, syntax 66). Three existing external-tool CLI tests
remain ignored; the 5,000-alias case was selected for a separate run. Log:
`/private/tmp/typaxis-production-structure-verification.log`.
The new tests cover actual VMB inline/block paint order, page-local MCIDs,
source text split over multiple pages, separate semantics for shared Forms,
explicit empty pages, same-input determinism, foreign borrowed owners,
changed semantics, and exact/one-less record/output/spool budgets.

The first focused run caught a test fixture's wrong page-master JSON path and a
wrong expectation that identical deterministic authorization values must be
rejected solely because they came from a separate preflight. The fixture path
was corrected; tests now distinguish equal authorization values from foreign
borrowed display/ledger owners and from changed semantic content. The corrected
focused cases and the regression command above passed.

This is marked **page contribution** evidence. It is not a final PDF, a
serialized StructTreeRoot/ParentTree, an independent extraction/render test, or
a full-book success. Final font/Form/object dictionaries, structure objects,
links/outline/anchors, terminal authorization and the public production writer
still require connection to this selected path. Formula-only extraction must
also be checked with the independent extractors, rather than inferred from
ActualText bytes around graphical paint. General subflows, authored speech,
formal VMB export, CID CFF/Harano, distinct 5,000-image/full-book gates and next
contract publication remain within the unchanged objective.

The separately selected 5,000-alias test now also builds the production structure
plan and marked page contributions. Result: **1 passed**, 230.81 seconds in this
debug test run (includes fixture setup and all upstream phases, not an isolated
structure benchmark). The assertions cover 5,000 admitted declarations, 5,000
selected paints, one shared Form, 5,000 distinct Formula owners with one group
each, page-local dense MCIDs, 5,000 occurrence ActualTexts and 5,000 Do commands.
No native-math/body font is inserted for this formula-only fixture. Explicit
`max_images=8192` is used. These are aliases of an actual VMB fraction SVG;
this does not establish 5,000 distinct images, default-config public check/build,
independent extraction or a final PDF. Log:
`/private/tmp/typaxis-production-structure-5000.log`.

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis \
  production_body_page_content_places_5000_real_svg_aliases_with_one_shared_form --locked
```

A final focused run also verifies a real vector Figure followed by its generated
Caption → P → text subtree. The Figure's MCR precedes the caption text's MCR on
the selected page, and the caption container has no invented paint/MCID.
`cargo test --manifest-path workspace/Cargo.toml --target-dir /private/tmp/typaxis-vmb-book-build -p typaxis-cli --bin typaxis production_body_structure --locked`:
**4 passed**, including the three earlier focused cases and this added caption
case. Log: `/private/tmp/typaxis-production-structure-focused.log`.
All test processes for this checkpoint reached exit status 0; no wait remains.

## Selected production PDF object contributions (2026-09-06)

`typaxis-pdf/src/production_body_objects.rs` now creates typed object
contributions from the exact borrowed marked body content. Indirect references
are `ProductionBodyObjectChunk::Reference(ProductionBodyObjectRole)` values,
separate from opaque byte chunks. The eventual final graph merger must resolve
these references, rather than search/replace number-like strings in binary font
programs or content streams. No absolute PDF object number, xref, catalog or
successful publication receipt is issued at this stage.

The contribution includes six objects per frozen body font (Type0, CIDFont,
descriptor, subset program, ToUnicode and CIDToGIDMap/CIDSet), selected SVG
Forms/ExtGStates, each page's marked content and its actual font/XObject resource
dictionary, StructTreeRoot, ParentTree, optional IDTree and source StructElems.
It uses the frozen TrueType/TTC/CFF body font plans, not a native-math font.
Page resources reference only fonts and Forms actually used on that page.
StructElem /K contains the selected MCRs in source-fragment order followed by
its registry children; ParentTree page arrays preserve the dense MCID order,
including empty arrays for blank pages. Alt and Lang remain occurrence/node
attributes. Full source text ActualText is not repeated on StructElems.

Every reference must resolve within the contribution except the typed Page
references, which the final page tree owner must supply. Link StructElems need
selected annotation OBJRs; until that navigation owner is connected, the builder
returns `PendingNavigation` instead of dropping annotation children. This is an
explicit remaining integration requirement, not a redefinition of the supported
final book model. Outlines/catalog/metadata/page dictionaries, navigation and
terminal/serialization authorization remain final-merge work.

Object records and copied bytes extend the existing marked-content budgets.
Temporary ToUnicode/CID-map buffers are bounded and conservatively charged as
well as their retained stream copies. The local max_pdf_objects check is a
lower-bound check on this contribution; the final merger must still check the
complete graph, including its pages/catalog/metadata/navigation objects, before
assigning numbers. Passing the local exact-count test does not imply a complete
PDF fits that same count limit.

Initial focused verification:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_body_objects --locked
```

Result: **3 passed**. Coverage: exact frozen TT/TTC/CFF subset programs,
ToUnicode scalar entries, font object references and format-specific descriptor
fields; actual VMB Form content and page-local resources; source-owner to
MCR/Page/MCID and ParentTree joins; repeated-input determinism and foreign
marked-content rejection; cumulative record/spool and local object-count
exact/one-less boundaries. These are object contribution tests, not independent
PDF extraction/rendering or full-book publication evidence.

The complete CLI/PDF regression command then passed:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-pdf -p typaxis-cli --lib --bin typaxis --locked
```

Result: **281 passed** (CLI 205, PDF 76), no failures, three existing external-tool
CLI tests ignored. The CLI run took 226.02 seconds, including the 5,000-alias
fixture's setup/upstream phases. No isolated performance improvement is claimed.
Log: `/private/tmp/typaxis-production-body-objects-verification.log`.
The large fixture now builds the typed object contribution and asserts the
selected relative vector object count, one StructElem per registry node, 5,000
ParentTree references in the exact group order, the selected page-content count,
and absence of fabricated native/body fonts. The existing one-Form/5,000-Do
checks remain active. This is still alias/explicit-limit/internal evidence,
not distinct 5,000/default-limit/public-build or complete-PDF evidence.

A subsequent focused run added blank-page ParentTree/resource checks and a
real internal-link fixture. The latter reaches selected marked content and is
then rejected with `PendingNavigation` because its annotation OBJR is not yet
supplied. The focused command above passed **4 tests** (three earlier cases plus
the added case), log `/private/tmp/typaxis-production-body-objects-focused.log`.
All processes started for this checkpoint completed with exit status 0.

## Selected body PDF assembly and independent extraction probes (2026-09-06)

Previous goal turn classification: progress. It corrected the design's historical
versus implemented defaults and added explicit body-ink/whitespace acceptance
criteria in both repositories. This checkpoint advances the selected PDF graph;
it does not complete the public production pipeline or reduce the full-book goal.

`typaxis-pdf/src/production_body_assembly.rs` now resolves typed body object
references and generates catalog, pages, Info, XMP, dense object numbers, byte
observations and classic xref. It checks the complete object count before
numbering, preserves stream bytes, verifies the exact contributing owner, and
charges cumulative records/spool/final output. Exact/one-less budget tests,
object offsets/hashes/references, foreign-owner rejection and repeated-byte
assembly are covered. Navigation with anchors/links/outlines remains explicitly
pending. The result is an inspectable diagnostic assembly, not a
`VerifiedPdfBytesReceipt`, publication authorization or final build manifest.
Its XMP omits the PDF/UA declaration; the existing completed writer's XMP
encoding retains that declaration and its regression tests pass unchanged.

A managed blank Type3 glyph supplies a position for each Formula ActualText.
It has no painting operators and uses text rendering mode 3. Its usage is inside
the Formula's MCID scope, in an inner Span carrying the occurrence ActualText;
it introduces no source node or extra MCID. Three managed PDF objects are
charged once, and only pages using anchors reference the managed font. The
existing selected body-font plans and shared SVG Form contents are unchanged.
Selected vector baseline is carried through the display projection. The anchor
matrix uses the selected viewport width/height and selected baseline, avoiding
an independently guessed baseline and an inexact width/height division.

Independent probing revealed why a parser/structure-only success was inadequate:

- Without positioned text, MuPDF warned that Formula ActualText had no position,
  and Poppler extracted no formula speech.
- A fixed 1 pt extraction height added an unwanted space between the equation
  and the following `B` in Poppler. The selected-height matrix fixes that case.
- Closing the ActualText scope while its font state is still active matters;
  the managed scope closes with `EMC`, then restores `Q`.
- **The explicitly authored post-formula space remains unresolved in Poppler.**
  The package contains `" B"`, the PDF contains its space glyph and correct
  advance, and MuPDF preserves it. Poppler 26.08.0 extracts `quartersB` instead
  of `quarters B`. The new verifier rejects this; no whitespace collapse is used.
  Experiments with arbitrary anchor heights and nested line-level ActualText
  were not adopted as a solution. The latter lost/duplicated content in the
  independent extractors. The PDF semantic/extraction boundary needs a proper
  solution before any public-build/full-book acceptance claim.

The existing normal TT/TTC body fixtures have empty glyph outlines. They remain
valid structural/subset/extraction tests, but cannot prove visible body text.
Two added mixed cases select the existing outlined Typaxis CFF Fixture (its A
is a synthetic triangle, not a Japanese-font specimen). A separate source-fixed
region for A and the second-page B confirm body ink independently of formula
ink. This does not replace the real Japanese-font/full-book gates.

Verification:

```sh
TYPAXIS_BODY_PDF_PROBE_DIR=/private/tmp/typaxis-body-assembly-20260906-final \
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_body_assembly --locked

cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis -p typaxis-pdf -p typaxis-display-list --lib --locked
```

Focused result: **2 passed**, including seven generated PDF cases and anchor
ownership/page/font-use assertions. Full Rust regression: **340 passed**
(CLI 208, display-list 56, PDF 76), zero failures and three existing ignored
external-tool tests. CLI duration 236.84 seconds includes the 5,000-alias
upstream fixture; it is not a distinct-5,000/public-build performance result.
Log: `/private/tmp/typaxis-production-body-assembly-regression.log`.

New `tools/verify_production_body_probe.py` uses pypdf, Pillow, Poppler and MuPDF
on the opt-in generated PDFs. Run it with the bundled dependency Python:

```sh
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_body_probe.py \
  --probe-root /private/tmp/typaxis-body-assembly-20260906-final \
  --output-root /private/tmp/typaxis-body-assembly-20260906-verification \
  --pdftotext /opt/homebrew/bin/pdftotext --mutool /opt/homebrew/bin/mutool
```

It records input hashes and exact tool versions (Poppler 26.08.0, MuPDF 1.28.2),
per-case/per-phase observations and failures. Source-fixed text expectations
preserve authored spaces; only the extractors' exact line/page framing differs.
This is a local diagnostic probe, not the versioned full-book/tool-policy runner.
It tests seven cases: TT, TTC, CFF text; formula-only; mixed body/formulas; outlined
CFF body/formulas; and an explicit post-formula-space case. It checks page
MCIDs/ParentTree/MCR/Alt, placement count and per-page managed font usage, plus
zero raster difference with the anchor glyph removed at 72/144/288 DPI.
Removing the anchor is a counterfactual for nonpainting, not an independent
reference for original SVG geometry. Full SVG-versus-PDF mask verification is
still required by the design.

Expected current result is **exit 1**, with exactly one failed check:
`vmb-body-spaced-cff` / extraction / Poppler. The other six extraction cases and
all seven structure/nonpainting checks pass. `observed.json` explicitly states
`public_build_verified=false` and `full_book_verified=false`. Negative probes
remove a Form placement, change an MCID, replace ActualText, make the managed
anchor painting, and remove body A; these must be rejected. Wrong ActualText is
also independently rejected by the text-extraction oracle.

Remaining work includes the authored-space failure above, selected navigation
and terminal/paint/manifest closure, public check/build integration, the formal
VMB exporter, all real-book/distinct-image/Harano/profile-publication gates and
the other pending rows at the top of this ledger. No full-book PDF was produced
or accepted by this checkpoint.

## Preserve selected body whitespace in PDF extraction (2026-09-06)

Previous goal turn classification: progress (diagnostic PDF assembly, independent
probes and a concrete failing authored-space case were committed). This follow-up
fixes that observed failure without changing text expectations, visible positions,
font subsets, SVG geometry or the full-book acceptance conditions.

`production_body_text.rs` records private byte ranges for the retained glyph and
font commands, separately from standalone q/Q and cluster ActualText wrappers.
The standalone text contribution is unchanged. `production_body_marked.rs` uses
those typed ranges to enclose the exact selected source-owner/line-fragment text
in one ActualText scope. It does not search/replace serialized PDF commands or
nest a group replacement around cluster replacements. The group is closed while
the last text paint's font/matrix is active, before its Q restores graphics state.
The Formula's existing independent ActualText/MCID/managed-anchor scope is retained.

Each body's replacement string is encoded incrementally from its selected draws;
no full source-node string is repeated on successive lines/pages. Its bytes use
the existing cumulative output/spool budget. Source ownership, MCID count/order,
font CIDs, glyph coordinates, vector Do count and Form sharing remain unchanged.
A multipage source-text regression now asserts each fragment's replacement text
and rejects repeating the whole source text on every page.

Primary-source investigation helped distinguish layout from extraction behavior:
[Poppler TextOutputDev.cc](https://skia.googlesource.com/third_party/poppler/+/master/poppler/TextOutputDev.cc)
ends words on standalone whitespace and uses font-dependent geometric heuristics
when dumping raw words; ActualText is processed using the graphics state active
at its end. Local PDF experiments, rather than this moving source alone, verified
the actual installed 26.08.0 behavior. The fix preserves explicit text instead of
retuning the managed font or choosing an arbitrary height to satisfy a heuristic.

Verification commands and evidence:

```sh
TYPAXIS_BODY_PDF_PROBE_DIR=/private/tmp/typaxis-body-selected-text-20260906 \
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_body_ --locked

cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build -p typaxis-pdf --lib --locked
```

The selected-body run passed **27 tests**, including exact budget boundaries,
multiline/multipage source text, the 65,535+ painted-glyph/CID case and 5,000 real
SVG aliases. Its 271.05-second duration is not a controlled performance result.
Log: `/private/tmp/typaxis-body-selected-text-focused.log`. The PDF library's
**76 tests passed**, log `/private/tmp/typaxis-body-selected-text-pdf-regression.log`.
A subsequent focused `production_body_assembly_graph` run passed after adding
an eighth PDF case with two explicit spaces on both sides of the formula.

```sh
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_body_probe.py \
  --probe-root /private/tmp/typaxis-body-selected-text-20260906 \
  --output-root /private/tmp/typaxis-body-selected-text-20260906-verification \
  --pdftotext /opt/homebrew/bin/pdftotext --mutool /opt/homebrew/bin/mutool
```

The independent probe now exits **0**: **eight cases pass all structure,
extraction and nonpainting checks**, and **seven negative assertions reject**
their mutations. Poppler 26.08.0 and MuPDF 1.28.2 preserve zero, one and two
post-formula spaces in these fixtures; their exact tool framing remains separate
from authored text. The added regression mutant removes only the body replacement
around `" B"`, preserving every glyph and vector placement. The independent
extraction oracle rejects it, recreating the prior failure rather than merely
checking for the new PDF syntax. The prior failed observations remain in the
previous output directory as historical evidence; their expected strings were
not changed to make the new run pass.

The seven pre-existing cases were also compared directly against their previous
PDF renders at 72/144/288 DPI: all **30 page-image comparisons were pixel-identical**.
The eighth case independently checks its source-fixed body-A region and zero ink
change with the managed anchor removed. These checks still do not substitute for
the original-SVG/reference masks, real Japanese font or full-book gates.

The observed authored-space failure from the preceding checkpoint is resolved
for this corpus. General line-breaking/bidi/whitespace policy and full-book
verification remain part of the original objective; no broader completion is
inferred from these eight small PDFs. Selected navigation, terminal/paint/manifest
closure and public build integration remain the next PDF integration work.


## Selected inline anchor positions and logical link bounds (2026-09-06)

Previous goal turn classification: progress (selected body ActualText fixed the
observed authored-space extraction failure and independent probes passed).
This checkpoint adds the retained positions needed by selected navigation;
it does not remove PendingNavigation or authorize public publication.

`ProductionPreparedInlineAnchor` records source owner/span and an ordered logical
unit gap without adding a glyph, advance or break opportunity. The line projection
retains same-gap source order, assigns a line-boundary gap to the following line
except at paragraph end, and uses actual unit pens plus the selected origin shift.
Markers on opposite sides of an explicit hard-break control remain distinct.
The marker-to-line pass is linear in markers plus selected lines.

`ProductionBodyInlineAnchor` borrows the exact selected marker and translates it
with its actual page fragment. It retains page/fragment identity, x and baseline,
separately from all text/vector draws. Per-line binary lookup selects only the
matching marker range. Preparation, line projection and page projection each
charge retained marker records before allocation against document-wide ceilings.
No per-paragraph budget reset or guessed page coordinates are introduced.

`ProductionBodyTextDraw::logical_bounds` uses selected cluster pen, summed real
glyph advances, selected page baseline and actual font ascender/descender. This
is logical interaction area, not glyph ink bounds. A zero width or height yields
no positive-area box; the code does not invent an extent or reject otherwise
valid text solely because its logical box is empty. Vector interaction area can
use the existing selected viewport. Link ancestry, per-line aggregation and
annotation construction remain subsequent work.

Private preparation/line/display algorithm identities advance to
`typaxis.production-inline-preparation/3`,
`typaxis.production-inline-line-layout/2`, and
`typaxis.production-body-display/2`. The flow/shape/selected fingerprint chain
binds source markers and geometry to these new projections. No public contract,
profile, capabilities schema or publication identity is added.

An attempted anchor-only paragraph regression exposed existing profile gates:
a wholly empty semantic container fails base preflight; an anchor-only paragraph
beside real content fails tagged preflight with UnsupportedSemantic. The final
regression verifies that rejection and that source flow still retains its two
anchors. It does not weaken the gate, insert dummy text or claim a successful
empty-paragraph selected placement. The line projection explicitly represents
unplaced markers for a future nonpainting flow-cursor implementation. The design
in §14.5 records both the profile and destination work still required.

Focused regression evidence covers duplicate gap markers, before/after hard
breaks, consecutive and terminal breaks, soft-break taken/not taken, no invented
A/B break, real VMB negative-origin compensation, exact shared document record
limits, unchanged text/glyph geometry, three-page marker translation, and logical
bounds on body/formula pages including explicit whitespace. The page case keeps
12 markers separately from its three text draws and verifies all actual fragment
identities; the marker record charge is included in the cumulative display total.

The next navigation owner must bind this exact display/structure, resolve block
and heading anchors to actual descendant fragments, aggregate Link children per
page/line, and provide typed destination/outline/annotation roles. ParentTree,
OBJR, catalog and page references must close before PendingNavigation is removed.
The full-book, Harano, exporter, terminal/paint/manifest and independent whole-book
gates remain unfinished. VMB exporter responsibilities and the anchor-only gate
are recorded in its requested `docs/typaxis-book-export-design.md` §15.16.


Verification commands:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_line_ --locked
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_body_ --locked
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_body_display --locked
TYPAXIS_BODY_PDF_PROBE_DIR=/private/tmp/typaxis-body-navigation-positions-20260906 \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_body_assembly_graph --locked
```

The line projection run passed **9 tests**; the body run passed **28 tests**,
including 5,000 real SVG aliases and 65,535+ selected body glyphs, in 225.99 seconds
(not a controlled performance measurement). Logs:
`/private/tmp/typaxis-body-anchor-tests.log` and
`/private/tmp/typaxis-body-anchor-regression.log`.
After review made zero-height logical boxes explicitly optional, the focused
page-display **2 tests** and diagnostic assembly **1 test** passed on the final
code. Logs: `/private/tmp/typaxis-body-anchor-display-tests.log` and
`/private/tmp/typaxis-body-anchor-assembly-probe.log`.

The workspace-wide `cargo fmt --all --check` found existing formatting differences
in unrelated files; those were not rewritten. The changed preparation, selected
line, display-body and inline test modules pass targeted rustfmt checks.
Both repository diffs pass whitespace validation. This checkpoint introduces no
GitHub Actions and performs no remote publication.


The final diagnostic PDFs also passed the independent probe:

```sh
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_body_probe.py \
  --probe-root /private/tmp/typaxis-body-navigation-positions-20260906 \
  --output-root /private/tmp/typaxis-body-navigation-positions-20260906-verification \
  --pdftotext /opt/homebrew/bin/pdftotext --mutool /opt/homebrew/bin/mutool
```

Result: **8 cases**, **0 failed checks**, **7 rejected negative probes**.
The structure/extraction/nonpainting results are in the output directory's
`observed.json`; log `/private/tmp/typaxis-body-anchor-independent.log`.
These existing small diagnostic PDFs have no selected link annotations and do
not establish navigation completion or the full-book acceptance condition.


## Selected navigation connected to diagnostic PDF objects (2026-09-06)

Previous goal turn classification: progress (private inline marker positions and
logical text bounds were committed and verified). This checkpoint connects those
positions to actual PDF navigation; it does not redefine the full-book objective
around a smaller diagnostic corpus.

`typaxis-display-list/src/production_navigation.rs` adds a private-field owner
borrowing the exact selected structure/display and their validated source flow.
Inline destinations use retained marker page coordinates; heading/container
anchors use the first actual selected descendant fragment. A reverse registry
pass propagates first fragments once. Parent-before-child lookup determines each
paint's Link ancestor, then unions logical text bounds and actual SVG viewports
only within one selected line/page. Source-order page ranges and node annotation
indices avoid rescanning all annotations per page or StructElem. Unplaced
anchors/links, nested links, receipt mismatch, geometry failure and resource
limits retain the responsible source owner in typed errors.

The flow now borrows typed internal/URI targets on BeginLink sites only. URI
strings have already passed the package SafeUri policy; they are neither
reconstructed from accessible names nor treated as destinations. EndContainer
has no target. `typaxis.production-text-flow/3` distinguishes the new flow
contract, while the package/flow/shape/selection fingerprint chain binds exact
source target bytes. Navigation has algorithm
`typaxis.production-body-navigation/1` and its own structure-bound fingerprint.

The PDF contribution owns that navigation result and adds its additional record
charges once to the common marked-content ancestor. Destinations are emitted as
UTF-16BE name-tree keys in encoded code-unit order. Internal annotations reference
those names; URI annotations use the existing writer convention of hex-encoded
original URI bytes. All annotations carry source accessible names, actual page
rectangles, page references, Border [0 0 0], Print flag and distinct StructParent
keys. Page MCID ParentTree arrays are retained; annotation keys follow page keys
and refer to Link StructElems. Each Link K array retains source children and adds
one matching OBJR for each selected annotation.

Outline topology retains dense source IDs, hierarchy, sibling backlinks and
open descendant counts. Typed roles for destination trees, outline roots/items
and link annotations join the full object graph before number assignment.
Assembly connects catalog Names/Outlines and page Annots, checks all references,
and retains its existing finite graph/output/spool limits. The old unconditional
PendingNavigation enum/rejection is removed; unsupported or unplaced source
content must fail at its actual typed owner boundary.

Regressions include a real VMB formula with surrounding link text, heading and
semantic-container destinations, root outline siblings and a child entry,
three-page internal and URI links, exact final object record limits, and rejecting
a different structure owner even for the same input. Source assertions prove the
last-page target belongs to its actual inline marker, container/heading targets
match real fragment top coordinates, and the clickable VMB rectangle contains
both body text and the full selected formula viewport. URI-only navigation has
no destination tree. The existing blank-page test now verifies a fully connected
internal annotation instead of expecting blanket navigation rejection.

`tools/verify_production_navigation_probe.py` independently parses diagnostic PDF
bytes with pypdf and compares them with the selected receipt export. It checks
name-tree ordering, page/rectangle conversion, target kinds, annotation ownership,
ParentTree/OBJR backlinks, outline parents/siblings/descendant counts and coverage.
Its three cases reject 28 mutations spanning missing/duplicate annotations,
shifted rectangles/destinations, missing targets, URI changes, page-key collision,
missing OBJRs, wrong ParentTree owners and an outline cycle. These are independent
PDF serialization/graph checks, not a viewer interaction, PDF/UA, public-build or
full-book certificate. The report explicitly keeps public_build/full_book false.

Remaining work includes the existing anchor-only paragraph profile/flow cursor,
all unconnected general pagination/source features, terminal and paint ownership,
manifest/public pipeline closure, formal RenderBook exporter, Harano CID CFF and
all original whole-book gates. No approximate source coordinates, dummy paints,
chapter splitting or weaker full-book success condition are introduced.


Review additionally bound navigation's preallocation checks to the complete
retained marked-content record charge. The caller supplies a base no lower than
the sealed structure's charge; the PDF contribution verifies it equals its exact
marked owner. An exhausted base and an understated structure base fail before
index/destination/link allocation. Final object budgets still include additional
navigation records exactly once. Focused tests cover these two boundary failures
in addition to the final object limit at N/N-1.

Verification commands and results:

```sh
TYPAXIS_NAVIGATION_PDF_PROBE_DIR=/private/tmp/typaxis-selected-navigation-20260906 \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_body_navigation --locked
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-syntax -p typaxis-pdf --lib --locked
```

The final navigation run passed **3 tests** (multiple PDF cases per test), log
`/private/tmp/typaxis-navigation-final-focused.log`. The library run passed
**76 PDF tests** and **66 syntax tests**, log
`/private/tmp/typaxis-navigation-library-regression.log`.

A broader `production_` run was initially invoked with both diagnostic probe
environment variables, exporting fresh body/navigation PDFs. It passed **51**
tests including the 5,000 real SVG alias and 65,535+ body-glyph cases, but **5 public
CLI tests failed** because strict public configuration correctly rejects unknown
`TYPAXIS_BODY_PDF_PROBE_DIR`/`TYPAXIS_NAVIGATION_PDF_PROBE_DIR` settings. That run's
346.77-second duration is not controlled performance evidence. Log:
`/private/tmp/typaxis-navigation-production-regression.log`. This is a test command
environment error, not justification to weaken unknown-configuration rejection.
Probe variables belong only on the selected diagnostic test filters above.
The public tests are rerun separately without these variables:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis machine_production_book_1_ --locked
```

The independently parsed diagnostic artifacts are verified with:

```sh
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_navigation_probe.py \
  --probe-root /private/tmp/typaxis-selected-navigation-20260906 \
  --output /private/tmp/typaxis-selected-navigation-20260906-verification/observed.json
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_body_probe.py \
  --probe-root /private/tmp/typaxis-body-navigation-connected-20260906 \
  --output-root /private/tmp/typaxis-body-navigation-connected-20260906-verification \
  --pdftotext /opt/homebrew/bin/pdftotext --mutool /opt/homebrew/bin/mutool
```

Navigation: **3 cases, 28 rejected mutations**. Existing body/formula probe:
**8 cases, 0 failed checks, 7 rejected mutations**. These preserve exact extraction,
structure and nonpainting regressions alongside the new navigation graph checks.
Logs: `/private/tmp/typaxis-navigation-independent.log` and
`/private/tmp/typaxis-navigation-body-independent.log`. The expected navigation
coordinates are exported from sealed selected receipts, with separate Rust
assertions tying those receipts to authored source/page/count facts. They are not
reference-render or whole-book visual oracles.


The clean public CLI rerun passed **5 tests**, including the existing combined
fixture and feature-local tamper matrix. Log:
`/private/tmp/typaxis-navigation-public-regression.log`. This verifies the existing
public profile regression; it does not route the new selected-body writer through
that public pipeline or prove the VMB full book succeeds there.

The final navigation oracle also compares each annotation's Contents exactly
with the source registry's accessible name. Three additional nonempty-name
mutations are rejected, giving **28** negative navigation probes in total. Final
receipt-export tests passed **3 tests**; a fresh selected-body assembly probe passed
**1 test** after the allocation-base review. Logs:
`/private/tmp/typaxis-navigation-final-focused.log` and
`/private/tmp/typaxis-navigation-final-assembly.log`. Public configuration rules
were not changed to accept diagnostic environment variables.


## Selected ordinary raster figures and captions (2026-09-06)

The supplied full book has **51 ordinary figure placements referencing 48 PNG
resources**, in addition to its SVG formulas. Their 39,708,000 source pixels would
require up to 158,832,000 RGBA bytes; the largest image is 1200 × 720. This is input
inspection, not a whole-book layout or peak-RSS result. Previously the selected
body paginator rejected the ordinary Figure region.

The production flow now retains each Figure's source owner/span, image ID, Alt,
placement and resolved ordinary style. Prepared figures bind the exact admitted
hash/dimensions and compute height from explicit physical width and pixel aspect
with one exact ties-to-even conversion. The common forward cursor places a
raster fragment followed by actual caption lines. keep_caption applies across
that real span; before/after spacing and keep_with_next use the first/last actual
fragment. Width overflow and oversize figures are diagnosed at the owner instead
of shrinking, splitting or clipping the image. Auto width, non-block placement,
vector media on this ordinary-raster path, named pages and unconnected caption
subflows remain explicit unsupported/pending cases. These private algorithms
advance their flow/preparation/selection/display/structure identities; no public
profile or capabilities claims are changed.

The selected display carries admitted image references. New raster plans derive
only from that display and share stable identical bytes/hash/media across logical
IDs while preserving every source Figure/Alt/caption/MCID. PNG color and alpha are
losslessly compressed with the already-locked flate2 1.1.9/miniz_oxide 0.8.9 Rust
backend; opaque masks are dropped. JPEG retains its admitted normalized stream
and ColorTransform. The existing resource freeze paths retain their encoding
policy; their shared decoder now reports allocation ceilings as ResourceLimit
and reserves its normalized output fallibly.

The raster branch accepts a retained base no lower than its exact font/display
owner and checks record/spool ceilings before its maps, decode buffers and bounded
compressed writer allocate. The page owner supplies the full font/text/vector
base and verifies the same base on receipt validation; marked content merges the
full raster charge without resetting it. PNG decoder allocations receive an
explicit ceiling. Two pixel buffers, that decoder ceiling, a 1 MiB allowance for
the pinned fixed deflater workspace, and temporary compressed copies are checked
as a peak, while only compressed retained payloads carry forward. The exposed
peak is a conservative accounting result, not measured RSS or a performance gate.
The fixed deflater uses bounded hash/code/Huffman buffers plus flate2's 32 KiB
output buffer; changing that backend requires reviewing its workspace allowance.

Page streams keep one root Y flip and apply `[w 0 0 -h x y+h]` for image scanlines.
PDF Image/SMask objects and page Resources use shared typed roles; each Figure
gets its own structure MCR and source Alt, and captions remain its children. Raster
Figures have no formula ActualText anchor and do not inject their Alt into body
text extraction. Final source order still comes from the selected display.

Fixtures now include the unchanged user-supplied image 39/node 170 PNG and its
package/hash provenance, plus a synthetic asymmetric 2 × 2 RGBA image. The tests
combine each of these and the existing baseline JPEG with actual VMB inline/block
fraction SVGs, actual caption/following text and a distinct-ID alias Figure.
They verify real caption keeps, explicit bounds, half-even ties, width/height
overflow, sharing, alpha objects, structure, deterministic PDF bytes, exact owner
and base validation, and record/peak-spool success at N and rejection at N−1.
The fixture Alt/caption labels are synthetic, not the book's authored speech.

Verification:

```sh
TYPAXIS_RASTER_PDF_PROBE_DIR=/private/tmp/typaxis-selected-raster-20260906 \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_body_raster_ --locked
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_ --locked
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-resources -p typaxis-syntax -p typaxis-pdf --lib --locked
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_raster_probe.py \
  --probe-root /private/tmp/typaxis-selected-raster-20260906 \
  --output-root /private/tmp/typaxis-selected-raster-20260906-verification \
  --mutool /opt/homebrew/bin/mutool --pdftotext /opt/homebrew/bin/pdftotext
```

The final focused run passed **3 tests**, including expanded boundary/overflow
cases (`/private/tmp/typaxis-raster-final-focused.log`). The broad run passed
**59 tests**, including 5,000 SVG alias placements and 65,536 selected body glyphs
(`/private/tmp/typaxis-raster-production-regression.log`). Its 244.48-second
concurrent local duration is not controlled performance evidence. The later
library run passed **76 PDF, 28 resources and 66 syntax tests**
(`/private/tmp/typaxis-raster-library-regression.log`). A 32-test selected-body
regression also passed before the final decoder/boundary review.

Independent pypdf validation and MuPDF 1.28.2 rendering passed **3 raster cases**,
with **17 rejected mutations**: missing/duplicate paint, shifted/flipped image,
wrong Alt and missing/changed alpha. Decoded image color/alpha matches source
pixels, matrices match selected physical placement and shared payloads retain
separate Figure/Alt/MCR/ParentTree owners. Both Poppler 26.08.0 **raw mode** and
MuPDF text extraction preserve the expected body/formula/caption source sequence,
removing only tool-generated line/page separators from that comparison. No
authored spaces are collapsed. This is not a claim about Poppler's default
geometric reordering mode. The VMB PNG's RGB stream is 50,359 compressed bytes;
its raster stage reports 66,471 retained bytes including its inherited base and
11,659,194 conservative peak bytes. Report:
`/private/tmp/typaxis-selected-raster-20260906-verification/observed.json`; log:
`/private/tmp/typaxis-raster-independent.log`.

Remaining scope is unchanged: all 48 original PNGs in the full book, 5,000
distinct mixed resources, general pagination/list/table/footnotes and final line
shaping, terminal/paint/manifest and public runner closure, formal RenderBook
exporter, original reference rendering, Japanese/Harano CID CFF and the complete
whole-book gates. The diagnostic probe reports public_build/full_book false.
VMB responsibilities and this boundary are recorded in its requested
`docs/typaxis-book-export-design.md` §15.18.


After the final decoder/budget review, the selected-body regression passed
**32 tests** (the already-verified 5,000-alias and 65,536-glyph cases excluded),
log `/private/tmp/typaxis-raster-final-body.log`. This run used only the
`production_body_` diagnostic filter with BODY/NAVIGATION probe directories;
those variables were not passed to public CLI tests. The final public profile
regression passed **5 tests**, log `/private/tmp/typaxis-raster-public-regression.log`:

```sh
TYPAXIS_BODY_PDF_PROBE_DIR=/private/tmp/typaxis-body-raster-connected-20260906 \
TYPAXIS_NAVIGATION_PDF_PROBE_DIR=/private/tmp/typaxis-navigation-raster-connected-20260906 \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_body_ --locked -- \
  --skip production_body_page_content_places_5000_real_svg_aliases \
  --skip production_body_more_than_65535
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis machine_production_book_1_ --locked
```

Fresh independent regressions passed **8 body cases / 7 rejected mutations**
and **3 navigation cases / 28 rejected mutations**:

```sh
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_body_probe.py \
  --probe-root /private/tmp/typaxis-body-raster-connected-20260906 \
  --output-root /private/tmp/typaxis-body-raster-connected-20260906-verification \
  --pdftotext /opt/homebrew/bin/pdftotext --mutool /opt/homebrew/bin/mutool
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_navigation_probe.py \
  --probe-root /private/tmp/typaxis-navigation-raster-connected-20260906 \
  --output /private/tmp/typaxis-navigation-raster-connected-20260906-verification/observed.json
```

Logs: `/private/tmp/typaxis-raster-existing-body-independent.log` and
`/private/tmp/typaxis-raster-existing-navigation-independent.log`. A final raster
focused rerun also verifies byte-identical frozen plans from an independent
encoding of the same selected owner; its 3 tests pass.


## 2026-09-06: selected list markers and item body/math frames

The preceding goal turn added the concrete list design to Typaxis §14.7 and
VMB §14.7. This turn connects the implementation through the diagnostic PDF
path. The preserved original package contains **16 lists / 58 items** (first
list owner 2908); that inventory is a full-book requirement, not a claim that all
original items have now passed layout.

- `typaxis-text/generated_overlay.rs` supplies a bounded stage-owned generated
  namespace. `typaxis-syntax/production_flow.rs` derives each canonical label from
  checked list start/order and binds its item owner, parent list, source/language
  and generated key. Parsed plus generated bytes and per-buffer limits are
  checked before marker allocation. Overflow reports the actual item owner.
- `typaxis-shaping/production_list_markers.rs` uses the admitted list face/size and
  real hhea metrics, glyphs and clusters. The namespace remains Generated, never
  a fabricated parsed span. Backend transient capacity uses the bounded shaping
  request ceiling; exact retained glyph/cluster counts share the document budget.
- `typaxis-layout/production_list_frames.rs` computes a maximum-advance marker
  column per list, end alignment, the existing 1 em gap, and nested item widths.
  Paragraph breaking and placement use the same frame. Line output records carry
  the frame/generated-record base instead of receiving a fresh allowance.
- `typaxis-pagination/production_list.rs` attaches each label once to the first
  actual paint fragment; forced breaks and nonpainting hard-break lines stay in
  the flow. Real label ascender/descent reserve leading/trailing height without
  changing the body/SVG fragment height. Nested labels sharing the first fragment
  use the maximum required height once. Raster widths and unnumbered block-SVG
  alignment are recomputed in the containing item frame; numbered blocks still
  require their separate number preparation and are rejected when unconnected.
- `typaxis-display-list/production_list.rs` projects generated clusters through
  the common font/CID/subset/PDF text path. Generated display IDs cannot collide
  with parsed IDs. `production_structure.rs` joins them to the exact generated
  Lbl, requires one complete label group per item and preserves L/LI/Lbl/LBody.
  ActualText is emitted once inside the Lbl MCID, not repeated on StructElem.

Internal identities change to source flow /5, authored text shape /3, body
pagination /3, body display /4 and body structure /3. The final generated store,
terminal/paint/manifest and public profile remain separate uncompleted owners.
The old unframed paragraph API cannot authorize a list layout: it lacks the
sealed list/item frames and pagination still rejects that path.

`production_list_tests.rs` covers ordered 9→10, bullets, nested widths, actual
TT/TTC selection, missing U+2022, marker number overflow, exact record boundaries,
first inline/block fraction, first raster/caption, continuation pages, large
markers, shared first-fragment nested labels, and leading hard/forced breaks.
Three derived no-MATH font fixtures and their generator/provenance live under
`vmb-book/list-fonts/`. The first two preserve original empty ASCII outlines and
advances while adding a real bullet; a separate visible fixture supplies authored
seven-segment decimal/dot outlines for paint probes. The old unmodified font is
retained as the missing-bullet negative. Original public fixture files are intact.

Verification commands (all local, target `/private/tmp/typaxis-vmb-book-build`):

```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build cargo test \
  --manifest-path workspace/Cargo.toml -p typaxis-cli production_list_ --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build cargo test \
  --manifest-path workspace/Cargo.toml -p typaxis-cli production_ --locked -- \
  --skip production_body_more_than_65535_selected_glyphs_use_one_cid_without_losing_occurrences \
  --skip production_body_page_content_places_5000_real_svg_aliases_with_one_shared_form
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build cargo test \
  --manifest-path workspace/Cargo.toml -p typaxis-text -p typaxis-syntax \
  -p typaxis-pagination -p typaxis-display-list -p typaxis-pdf --lib --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build cargo test \
  --manifest-path workspace/Cargo.toml -p typaxis-shaping -p typaxis-layout \
  -p typaxis-resources --lib --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  TYPAXIS_LIST_PDF_PROBE_DIR=/private/tmp/typaxis-selected-list-20260906 \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli \
  production_list_independent_pdf_probes --locked
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_list_probe.py /private/tmp/typaxis-selected-list-20260906 \
  /private/tmp/typaxis-selected-list-20260906-verification
```

The focused list run passed **11 tests** (`/private/tmp/typaxis-list-final-focused.log`).
The production regression passed **67 tests** before the final additional shared-
first-fragment test, which passed in the 11-test run. The earlier body regression
passed **34 tests**, including 65,536 body glyph occurrences and **5,000 real-SVG
aliases**, in 233.68 seconds; this concurrent local duration is not a controlled
performance measurement. Library regressions passed **text 6, syntax 66,
pagination 85, display 56, PDF 76, shaping 24, layout 65 and resources 28**.
Logs: `/private/tmp/typaxis-list-production-final.log`,
`/private/tmp/typaxis-list-body-regression.log`,
`/private/tmp/typaxis-list-library-regression.log`,
`/private/tmp/typaxis-list-shaping-layout-regression.log`.

The independent list verifier passed **4 cases / 7 visible labels / 36 rejected
mutations**: nested lists, block-first, inline-first and raster-first items.
It parses MCIDs, Lbl/LBody order, list numbering attributes, ParentTree/MCRs,
exact replacement strings and glyph positions independently. It compares exact
Poppler raw and MuPDF text, preserving tool-specific line/page framing instead
of collapsing authored whitespace. MuPDF rendering at 144 dpi is compared to a
counterfactual with label glyph painting removed: every label contributes ink,
and no pixels outside the label areas change. Report:
`/private/tmp/typaxis-selected-list-20260906-verification/observed.json`; log:
`/private/tmp/typaxis-list-independent.log`. Decimal outlines are diagnostic,
not a substitute for publication/Japanese font reference rendering.

Review and failed-run evidence: the initial inherited font lacks U+2022, so a
bullet fixture was added rather than substituting a character. Nested geometry
exposed an actually too-narrow positive fixture; its width is now explicitly
sufficient and narrow input remains a negative. The budget test then found an
incorrect coupling between harfrust's minimum 16,384 temporary records and the
much smaller retained document output allowance; it now follows the body-run
budget separation and passes the exact/max-minus-one boundary. A separate review
found labels attached to empty hard-break lines; the first-paint lookup and its
forced-page regression fix that. The verifier initially expected StructElem
ActualText; it now enforces the specified single marked-content replacement
owner and rejects a duplicated structure-level replacement.

Still required: numbered math and other generated text, full generated-store
convergence, tables/footnotes/general pagination/final-line shaping, public
terminal/paint/manifest and runner closure, formal VMB RenderBook exporter and
semantic speech, 5,000 distinct mixed images/8,192 positive boundary, Japanese
and unchanged Harano CID-CFF/IVS, original reference rendering, actual one-package
full-book PDF and cross-host/performance gates. None is replaced by these selected
list probes. VMB changes/boundaries are recorded in its requested docs §15.19.

Final existing-probe regression passed **32 body tests**, then independent
verification passed **8 body cases / 7 rejected mutations**, **3 navigation cases
/ 28 rejected mutations**, and **3 raster cases / 17 rejected mutations**.
The probe environment was supplied only to `production_body_`, never to public
CLI tests. The raster verifier received a subdirectory containing the three
unchanged raster PDF/expected pairs, since the common probe output also contains
navigation expected JSON with a different schema.

```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  TYPAXIS_BODY_PDF_PROBE_DIR=/private/tmp/typaxis-list-regression-probes \
  TYPAXIS_NAVIGATION_PDF_PROBE_DIR=/private/tmp/typaxis-list-regression-probes \
  TYPAXIS_RASTER_PDF_PROBE_DIR=/private/tmp/typaxis-list-regression-probes \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_body_ --locked -- \
  --skip production_body_more_than_65535_selected_glyphs_use_one_cid_without_losing_occurrences \
  --skip production_body_page_content_places_5000_real_svg_aliases_with_one_shared_form
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_body_probe.py --probe-root /private/tmp/typaxis-list-regression-probes \
  --output-root /private/tmp/typaxis-list-body-independent \
  --pdftotext /opt/homebrew/bin/pdftotext --mutool /opt/homebrew/bin/mutool
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_navigation_probe.py --probe-root /private/tmp/typaxis-list-regression-probes \
  --output /private/tmp/typaxis-list-navigation-independent/observed.json
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_raster_probe.py --probe-root /private/tmp/typaxis-list-regression-probes/raster \
  --output-root /private/tmp/typaxis-list-raster-independent \
  --pdftotext /opt/homebrew/bin/pdftotext --mutool /opt/homebrew/bin/mutool
```

Logs: `/private/tmp/typaxis-list-existing-probes.log`,
`/private/tmp/typaxis-list-existing-body-independent.log`,
`/private/tmp/typaxis-list-existing-navigation-independent.log`,
`/private/tmp/typaxis-list-existing-raster-independent.log`. All observations
remain diagnostic-stage evidence; no public/full-book success is asserted.


## 2026-09-06: CFF admission and TTC diagnostics through public check/build

The previous design-confirmation turn made documentation progress and rechecked
the actual package/font hashes. This implementation turn connects the detailed
font failure path to resource admission and both public runners. It does not
change accepted CFF/1 input, enable CID-keyed Harano, or complete the full-book gate.

`typaxis-font` now exposes `admit_sfnt_cff1_detailed`. Its fixed-size `Copy`
context records the first failure's phase, table, reason, requested face,
embedding inspection, available offsets, CFF operator/cmap format and relevant
admission budget observations. The legacy function calls the same implementation
and returns the original error kind; the accepted fixture's complete admission
receipt remains equal. No font data is rewritten to obtain admission.

The scanner captures positions directly in the original bytes. `offset_kind=field`
means an identified field/operator; `offset_kind=context-start` means the start of
the parsing unit, not a claimed exact offending operand. Unknown positions remain
absent. OS/2 is inspected for permission only after its range and checksum pass;
unsafe or unreadable OS/2 stays `embedding=not-checked`. Table/checksum/permission,
unsupported cmap and DICT operators, table/glyph/subroutine budgets are separated.
Nested charstring/subset diagnostics are not yet wired to these reserved fields.

Resource admission preserves detailed CFF errors. A separate diagnostic-only
container inspector explains invalid face indexes and CFF/CFF2 outline mismatch
without granting a receipt. TTC header/directory inspection is bounded to 4,096
faces; normal notes show at most 32 present indexes, count and truncation. Presence
is explicitly distinct from admission; truncated headers do not invent a list.
CLI diagnostics keep the package JSON Pointer, add resource URI and font notes,
and describe supported outlines plus the existing `inspect-font FONT` command.
R7100 appears once; existing R713x budget and I9190 mappings remain kind-derived.

The actual Harano probe exposed a second bug: partial resource progress discarded
production media declarations in the failed manifest. This turned an R7100 input
failure into exit 4 with `PackageResourceMismatch` when earlier fonts had been
admitted. `resource_progress_records` now checks declared/observed media and keeps
those facts for partial fonts and images. It still rejects mismatches and never
adds the failing resource. Regression tests assert exit 1, output-null failed
manifest, exact partial resource counts and retained media declarations, including
an SVG failure after admitted PNG/SVG resources.

Verification (target directory `/private/tmp/typaxis-vmb-book-build`):

- `cargo test --manifest-path workspace/Cargo.toml -p typaxis-font --lib --locked`:
  21 passed (`/private/tmp/typaxis-font-detailed-final.log`). Includes all truncated
  prefixes, legacy kind/receipt parity, VORG versus permission, corrupt OS/2,
  cmap 14, optional table ownership, exact CFF operator offsets and admission budgets.
  The added subroutine-budget test initially assumed the small fixture contained
  multiple subroutines and failed; it now explicitly constructs an over-budget
  INDEX count and checks rejection before object allocation/later offsets. This
  is not a claim of positive subroutine-corpus coverage.
- `cargo test --manifest-path workspace/Cargo.toml -p typaxis-resource-admission --lib --locked`:
  59 passed (`/private/tmp/typaxis-font-resource-tests.log`), including bounded TTC
  notes and preexisting admission, SVG safety and sharing regressions.
- `cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis machine_book_ --locked`:
  6 passed, 1 independent navigation test ignored
  (`/private/tmp/typaxis-font-book-regression.log`). The font test covers five
  negative cases through each runner and checks manifest publication as well as
  code, message, resource pointer and notes.
- `cargo test --manifest-path workspace/Cargo.toml -p typaxis-manifest --lib --locked`:
  38 passed (`/private/tmp/typaxis-font-manifest-regression.log`).
- A final combined font/resource rerun also passed 21 + 59 tests after the
  position-role review (`/private/tmp/typaxis-font-resource-final.log`).
- The earlier broad CLI/resource run passed 229 CLI tests (3 ignored) and 59
  resource tests, including the 5,000-alias selected-PDF regression
  (`/private/tmp/typaxis-cff-integration-tests.log`). This precedes the subsequent
  manifest fix; the focused book and manifest suites above verify that fix.

The new reproducible real-file negative gate is:

```sh
python3 tools/verify_vmb_font_diagnostics.py \
  --typaxis /private/tmp/typaxis-vmb-book-build/debug/typaxis \
  --font /Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  --output /private/tmp/typaxis-harano-diagnostic-20260906-final
```

The output directory must be new. The verifier requires the recorded Harano hash,
stages its unchanged bytes in the existing small production fixture, runs both
public commands with one package/config, records observations even on assertion
failure, and checks no PDF and a valid failed manifest. The successful report is
`/private/tmp/typaxis-harano-diagnostic-20260906-final/observed.json` with
`passed=true`, `harano_supported=false`, `full_book=false`. Both commands return 1
with `cff1 unsupported_table`, `/resources/font_faces/2`, `table=VORG`,
`font_byte=108`, `embedding=allowed`, `fs_type=0x0000`. The initial failed-manifest
finding is preserved under `/private/tmp/typaxis-harano-diagnostic-20260906/`.

Remaining font work includes selected-glyph/evaluator/subset context, detailed
TrueType metadata failures, and the entire CFF/2 CID/FD/cmap14/IVS and publication
scope in design §7/§9.1. The formal VMB exporter and real full-book gates remain
open; this negative admission result cannot stand in for them.

## 2026-09-06: bounded measured body page-end choices

The common body cursor now chooses page ends using the measured Item stream in
`typaxis-pagination/src/production_breaks.rs`. This replaces its greedy automatic
cut only; explicit page breaks, hard keep chains, real line/vector/raster/caption
heights and list marker extents remain binding. See design §14.8 for the exact
internal `typaxis.production-body-break/1` costs. The selected layout identity is
now `typaxis.production-body-pagination/4`; its fingerprint includes the policy,
all candidates, component costs, selected index and termination reason. Private
immutable decisions are exposed read-only for subsequent trace integration.

An overflowing page enumerates every fitting non-keep boundary. It rejects the
candidate at limit+1 before evaluating or allocating it instead of returning a
truncated best effort. EOF and forced breaks that already fit have only their
mandatory candidate. Widow/orphan costs count actual lines on the current page,
including continuation paragraphs; heading isolation and unused height use the
same real placement geometry. Materialization verifies its page ranges and used
heights against the chosen candidates. Decision/candidate/paragraph/heading
records share the existing cumulative record budget. The existing exact budget
case consequently changes from 33 to 40 (2 paragraph-length, 2 decision and 3
candidate records); 39 fails at the final owning paragraph as expected.

New verification includes:

- Seven kernel tests: first/last single-line avoidance, continuation counts,
  heading boundaries, hard keep exclusion, spaces and marker extents, exact
  candidate limit and no allocation at limit+1, mandatory blank pages, equal-cost
  deterministic tie choice, and fingerprint coverage of unselected candidates.
- Actual shaped four-line body: a fullest 3+1 split becomes 2+2, source text stays
  `AABB`, PDF text paints use pages 0,0,1,1 and limit 2 rejects the third candidate.
- A 70-line input with a valid 33-line page: default 32 rejects observed 33;
  explicit 128 retains all 70 source lines in 33+33+4 pages. An initial test used
  a body taller than its fixture media/trim and correctly failed BaseProfile;
  the test now explicitly defines the larger page and matching trim.
- Four actual-engine VMB fraction placements surrounded by text: 2+2 pages,
  one shared SVG Form, four usages, two Formula marked-content groups per page,
  nonempty per-occurrence Alt/ActualText and unchanged `A B` surrounding text.
  The first repeated fixture incorrectly reused source spans and failed
  InvalidSourceSpan; it now stores four original TeX spans and corresponding
  mapped buffers. No source validation was weakened. This is selected page
  content/structure evidence, not an independent raster comparison or public
  whole-book publication gate.

Commands ran locally with
`CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build` and `--locked`:

```sh
cargo test --manifest-path workspace/Cargo.toml -p typaxis-pagination --lib --locked
cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --locked
cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --locked \
  production_body_ -- --skip more_than_65535 --skip places_5000
```

Results: 92 pagination tests passed
(`/private/tmp/typaxis-body-break-pagination.log`); the full CLI run passed 233
with 3 ignored in 227.44 seconds
(`/private/tmp/typaxis-body-break-cli-full.log`), including the 65,536-glyph and
5,000-real-SVG-alias regressions. After a checked conversion of the decision
start index and the additional kernel tie test, the final focused CLI run passed
35 tests (`/private/tmp/typaxis-body-break-cli-final-focused.log`), and the final
pagination run above includes all seven new kernel tests. No public full-book
runner or optional independent PDF probe was invoked by this change.

The VMB design §8 now explicitly includes `max_page_break_lookback = 128`; §15.21
records ownership and no implicit retry/increase. This is a book export policy
proposal tested with the 70-line case, not an increase to the general default or
a proof that 128 suffices for every real full-book page. Formal VMB config and
RenderBook process integration, actual per-page candidate distributions and RSS
measurements remain pending.

The internal choices do not mint generic `PageBreakSearchBudget`, FlowPosition,
LayoutPassCoordinator state or terminal/paint authorization receipts. Connecting
those genuine owners, generated reference feedback and final line reshaping,
tables/footnotes and public runner/manifest publication remains required. There
is still no successful full-book PDF, 5,000 distinct mixed-resource gate,
8,192 positive boundary, or unchanged Harano CID-CFF/IVS acceptance evidence.

## 2026-09-06: VMB production math adapter and occurrence export binding

The previous goal turn was progress: the measured page-end candidate policy was
implemented and committed as Typaxis `cc64a49`, with VMB design `c3efbc10`.
Both worktrees were clean at the start of this work. This turn connects VMB's
existing actual production adapter to the previously implemented SVG lowering
and source projection in the new `math_text.go`, `resources.go` and
`math_export_seal.go`. It does not replace the full RenderBook/PDF objective with
a math-only exporter. The exact VMB API/ownership/limits and remaining work are
recorded in the required VMB document §15.22.

The new session validates prepared book/source-map identity through
`render.NewMathV2Adapter`, preserves each occurrence's actual TeX, semantic
speech, language, engine identity and original location, and emits typed
inline/block wire nodes. It shares only the derived SVG bytes by content hash;
different speech/source occurrences retain separate source slices, node IDs and
sidecar records. A different explicit font size can produce a separate image.
Numbered blocks require an explicit resolved number presentation, with a child
projection owner and exact TextSpan; missing numbers are rejected, not omitted.
Generic math speech is rejected by the new VMB-R1515 constant, while adapter,
source-map, geometry and cancellation causes remain available via Unwrap.

Failures poison the session. Finish requires a closed validated projection and
returns no partial successful set. Cumulative image/retained-byte/occurrence/JSON
budgets bound construction; JSON metadata is preflighted without allocation and
record-wise seals avoid reserializing a whole book for verification. A decoded
sidecar is not a completed in-process export authority. Before package output,
one batch `ValidateNodes` checks every inline/block against the completed set,
including source/geometry/meaning/spacing/number fields and omission/duplication.
The SVG-only registry must still be extended for PNG/JPEG in one global ID order;
there is no geometry cache that bypasses per-occurrence validation.

The tests build the existing `verified-math-intro` example through its real
BuildCanonical→Prepare path and use the production math engine. They cover
four math occurrences, speech-dependent raw hashes with shared derived SVG,
font-size variants, Japanese/non-BMP source text, generated number ownership,
source tamper, missing speech, exact small resource boundaries, metadata bounds,
poisoned/zero/finished sessions, cancellation causes and sidecar/wire tampering.
The final review also added source-order checks within each wire node list,
transferred private image buffers without a second retained copy, and verified
that canceled adapter cleanup can be completed with a live context without
resuming construction or invalidating already completed output.
This example is not the user-provided full fractions book.

A separate opt-in public admission test places producer-returned inline/block
nodes in the existing Typaxis test envelope (fixed metadata/page/font; complete
body traversal is not used). Initial test-envelope failures correctly rejected
an aliased config/resource root and an unknown document-root `kind` field. The
fixture now uses a config outside its private job and the correct root schema;
no Typaxis admission/schema/source check was loosened. Its final check-package
succeeds without diagnostics: 2 SVG resources, 2 math placements and 62 projection
bytes. No public build-package or independent PDF comparison was run for this
new VMB component.

Final local verification from `vmb-core/`:

```sh
VMB_TYPAXIS_CLI=/private/tmp/typaxis-vmb-book-build/debug/typaxis \
VMB_TYPAXIS_FIXTURE_ROOT=/Users/kazuyoshitoshiya/t/typaxis/samples/machine-package/profiles/production-book-1/combined/job \
GOCACHE=/private/tmp/vmb-typaxis-go-build \
go test ./internal/rendertypaxis -count=1 -v
GOCACHE=/private/tmp/vmb-typaxis-go-build go vet ./internal/rendertypaxis
```

The final test log is `/private/tmp/vmb-typaxis-math-export-final.log`; 18 top-level
tests passed, including the explicit public check (it is skipped without both
external-tool variables). `go vet` exited 0. The checked binary was rebuilt with
`cargo build --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --locked`
and `CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build`.

Recorded public check identities:

- Binary SHA-256: `96c5b910456bbc3799e48e8987754578a1ee53b44fa5f30d669d381f7e1d9eb3`.
- Package SHA-256: `1fdf330024ec00b3791f23dc65860647a05e53ea67011dd35cc8e2bc60486d96`.
- Source SHA-256: `8dfd8d8f7aa8364e532be7f5fbce03a103687020444a707029520430f300d40f`.

The full traversal/package writer and formal VMB process/ArtifactSink renderer,
Typaxis generic convergence/terminal/paint/manifest connection, unchanged Harano
CID-CFF/IVS, 5,000 distinct mixed images, 8,192 positive boundary and actual
full-book check/build/render/extract/structure gates remain required and open.

## 2026-09-07: remaining SVG locations and positive image-count boundaries

Safe-SVG 2 now retains the original `points` token and byte range for polygon/
polyline invalid numbers, coordinate overflow and incomplete coordinate pairs.
Count and IR construction use one allocation-free point visitor; construction
counts before reserving its exact segment array. Segment-budget failures retain
both the source pair and the generated segment ordinal. An implicit polygon
close identifies the end of `points`, without inventing a source `Z` command or
path index. Rectangle/ellipse derived extent failures retain the responsible
width/height/radius attribute. Frozen SVG1 grammar and accepted IR are unchanged.

Lexical preflight reports the original byte range for BOM, forbidden controls,
declarations, processing instructions and entities. The scanner retains a known
start-element name through duplicate attributes, invalid terminators and EOF,
so the outer scanner can attach its actual preorder/path indexes. A malformed
closing tag is not assigned the next opening element's index. Unknown owners
remain absent. Allocation and receipt failures are preserved through the V2
error adapter rather than relabeled as malformed input.

Public check/build tests cover invalid point tokens, out-of-range coordinates,
missing coordinates, comments and controls. Both commands report the same
R7100 reason, original `/resources/images/2` pointer and resource-local notes;
the package byte offset remains null. The failed manifest keeps the two earlier
admitted images, has no output and publishes no PDF.

The positive count boundary now runs both public commands at **8,192** image
declarations with production defaults and **1,024** with an explicit override.
Both build PDFs and retain all declaration records in their manifests. The
existing matching **8,193 / 1,025** rejection tests also pass. These are alias
**declaration-count** boundaries, not claims of 8,192 distinct placed images.

The selected-body test helper now uses the resolved config's M4 limits instead
of always substituting generic M4 defaults. Large SVG tests resolve the actual
production profile with an explicitly empty environment and no overrides.
A supplemental synthetic corpus keeps the actual engine fraction outlines and
changes the fixed fill to 5,000 different RGB values. This creates different
SVG hashes, IRs and Forms; it does not pad XML whitespace to defeat deduplication.
The corpus is intentionally distinct from the required §8.5 template
`x_i = i/(i+1)` generated by the engine, and cannot close that acceptance gate.

Local verification uses `--manifest-path workspace/Cargo.toml`,
`--target-dir /private/tmp/typaxis-vmb-book-build` and `--locked`:

- `cargo test ... -p typaxis-resource-admission --lib`: 62 passed, including
  original SVG1/V2 corpus, real VMB SVG, detailed positions and budget boundaries
  (`/private/tmp/typaxis-vmb-svg-detail-final.log`).
- `cargo test ... -p typaxis-cli --bin typaxis machine_book_svg`: 3 passed,
  including the new public diagnostic matrix
  (`/private/tmp/typaxis-vmb-public-svg.log`).
- `cargo test ... -p typaxis-cli --bin typaxis machine_book_image_count`:
  2 passed, 42.42 seconds (`/private/tmp/typaxis-vmb-image-boundary.log`).
- `cargo test ... -p typaxis-cli --bin typaxis -- --skip places_5000`:
  234 passed, 3 ignored, 2 filtered, 22.60 seconds
  (`/private/tmp/typaxis-vmb-cli-regression.log`). The two large resource tests
  run separately; ignored independent-tool tests are not counted as verified.

Final focused verification after the closing-tag index review and sharing the
exact same CommonOptions between the positive check/build boundary commands:
`cargo test ... -p typaxis-cli --bin typaxis machine_book_` passed **8**, ignored
**1**, in 19.36 seconds (`/private/tmp/typaxis-vmb-book-final.log`). The final
resource-admission rerun passed all **62** tests in 0.52 seconds.

The separate command
`cargo test ... -p typaxis-cli --bin typaxis production_body_page_content_places_5000 -- --nocapture`
passed **2** tests in **295.88 seconds**
(`/private/tmp/typaxis-vmb-5000-resources.log`). Each case admits and places 5,000
declarations with the resolved production defaults; alias case: 1 content hash /
1 Form, synthetic paint variants: 5,000 hashes / 5,000 Forms. Both assert 5,000
ordered usages, one Formula group and ActualText per occurrence, page-local MCIDs,
ParentTree references, balanced marked content and source-ordered paint. These
are selected page-content/object contributions, not public whole-book PDFs or
independent rendering/extraction evidence.

The external `/usr/bin/time -l` wrapper reports 314.22 seconds including compilation
but exits 1 because its `sysctl kern.clockrate` query is denied in the sandbox;
Cargo's own result is successful. No peak-RSS result was obtained. A read-only
2-second sample of this task's test process found active SHA-256 work in
navigation/structure receipt verification; it does not establish quadratic
complexity or a scaling baseline. No release performance claim is made.

The first added parser tests accidentally used the legacy broad-error test
adapter and failed their detailed-variant assertions; they now call the real
detailed work-budget entrypoint. An initial CLI assertion compared the custom
TestJson value with String and did not compile; it now compares string views.
No admission rule was relaxed to pass these tests.

Full-book public convergence/terminal/paint/manifest integration, the formal VMB
exporter, engine-generated 5,000 distinct formulas with mixed PNG, unchanged
Harano CID-CFF/IVS and independent full-book render/extract/tag acceptance remain
open. This checkpoint does not complete design 28.

## 2026-09-07: nested semantic-container widths in common body placement

The previous goal turn made implementation progress on SVG diagnostics and
resource boundary evidence; its changes are still present in the current
worktree. The full-book objective remains active and unchanged.

The measured body frame traversal now propagates semantic-container start/end
indents through nested content and restores the parent frame on exit. Paragraph
line selection, block SVG alignment, raster width/caption and list marker/item
frames use the same actual containing width. An exhausted container reports its
owner with `ContainerFrameExhausted`. Pagination still rejects nonzero container
indents when passed legacy arbitrary-width line results without measured frames.
A `typaxis.production-body-frames/1` domain now binds the frame fingerprint;
existing public profile/contract identities are unchanged.

Four new regressions cover nested source-order restoration, actual line reflow
with exact text preservation, block SVG centering in the nested width, tagged PDF
object assembly, raster/caption/list frame inheritance, body-fitting but
container-overflowing images, exhausted container width and rejection of
unframed precomputed lines. The focused `container` filter also runs one existing
semantic-container public regression: **5 passed**, 0 failed, 1.13 seconds
(`/private/tmp/typaxis-container-final.log`).

These are shared-placement and diagnostic-object tests, not an independent
full-book PDF acceptance result. Numbered block math in reduced frames, named
pages, final line reshaping/generic convergence and public terminal/paint/manifest
closure, formal VMB export and unchanged Harano acceptance remain required.

Broader local regression:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-layout -p typaxis-pagination -p typaxis-cli --lib --bins --locked \
  -- --skip places_5000 --skip more_than_65535
```

Result: CLI **237 passed, 3 ignored, 3 filtered**; layout **65 passed**;
pagination **92 passed** (394 passed total), no failures. Log:
`/private/tmp/typaxis-container-regression.log`. The three explicitly filtered
large resource/glyph tests were exercised in earlier checkpoints; this container
change does not claim a new execution of them or of ignored external validators.
All verification commands for this checkpoint completed. No full-book success
or new public capability is asserted.

## 2026-09-07: actual common-body block math terminal closure

Added a consuming finalization stage from actual common-body fragments to the
existing `StagingMathVectorTerminalLedger`. It verifies the block preparation's
registry and layout epoch, selected limits and fragment geometry, consumes each
block math flow once and requires the ledger to finish without missing flows.
The original placement and genuine terminal-set fingerprints are bound under
`typaxis.production-body-terminal/1`; a second finalization is rejected. A
prepared equation number does not establish selected/painted number completion,
so numbered blocks remain explicitly pending at this gate.

Ledger records use the cumulative fragment budget. A conservative preflight
spool bound includes terminal and registry-integrity serialization; actual
retained canonical JCS bytes carry into downstream font/content/object/PDF spool
accounting. Underlying terminal errors retain their typed cause.

Five focused CLI tests pass through real admission and preparation: actual
terminal/placement/downstream binding, forced-page and empty-registry handling,
foreign-registry rejection, exact cumulative record/spool boundaries, and
rejection of a genuinely prepared but unselected equation number. Initial test
fixtures omitted page-break classes or used an empty equation-number source
span; those invalid fixtures were corrected without relaxing production checks.
Log: `/private/tmp/typaxis-body-terminals.log`.

This stage closes the common placement's block-math ledger only. It does not
provide generic convergence, final reshaping, paint authorization, publication
or a public manifest. The full-book, formal exporter, real-engine 5,000 distinct
formulas/mixed PNG, unchanged Harano CID-CFF/IVS and independent acceptance gates
remain open. The full implementation goal remains active.

Broader local regression:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-layout -p typaxis-pagination -p typaxis-resources -p typaxis-cli \
  --lib --bins --locked -- --skip places_5000 --skip more_than_65535
```

Result: CLI **242 passed, 3 ignored, 3 filtered**; layout **65 passed**;
pagination **92 passed**; resources **28 passed** — **427 passed** total,
0 failures. Log: `/private/tmp/typaxis-terminal-regression.log`. Large resource
and glyph tests previously validated are explicitly filtered here; ignored
external validators and full-book public acceptance were not run by this command.

## 2026-09-07: selected and painted authored equation numbers

The preceding terminal-only checkpoint `33ca445` changed authoritative code and
passed local regression; it was progress. The current checkpoint connects the
previously missing numbered block atom through actual common-body selection,
display, shared text-font finalization, structure, marked content and diagnostic
PDF object assembly. It does not complete the full implementation goal.

The consuming terminal finalizer now borrows the original math registry and
selects number rectangles from the actual parent fragments before closing each
atomic terminal. Parent/child owners, page/fragment indices, shape identity and
rectangles are bound by `typaxis.production-body-terminal/2`. Reduced container
and list frames recompute the right edge and minimum gap without shifting the
formula to conceal a collision. Number placement records use the same cumulative
budget. Display refuses numbered blocks if genuine number selection is absent.

Number glyphs use the actual admitted hhea ascent/descent, signed half-leading,
visual run origins and the correct sign for vertical glyph offsets. The frozen
staging PDF number recipe is unchanged. Number draws enter the existing body
font/CID path and retain their own source spans and sealed shape association.
The source equation-number binding authorizes a distinct Span, with a single
marked-content replacement and no duplicate structure-level ActualText. Internal
display/structure identities advance to /5 and /4 respectively.

Focused tests cover parent/number atomic placement, missing-selection refusal,
forced-page retention, reduced-frame right alignment and collision rejection,
actual-metrics baseline, independent Span/ParentTree ownership, four-character
replacement and repeated-glyph CID sharing. The first new test incorrectly
assumed the number used font 0; the resolved style selects face 2. It now checks
against the real shape. A collision test originally failed on unrelated inline
line fitting; it now isolates the block. The first structure bridge assumed
number text lived in the node ActualText field; existing semantics correctly
keep it in the dedicated equation-number binding, and the bridge now uses that
binding without changing the frozen registry.

Independent fixed-probe verification:

```sh
TYPAXIS_NUMBER_PDF_PROBE_DIR=/private/tmp/typaxis-number-probe \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build -p typaxis-cli --bin typaxis \
  production_body_equation_numbers --locked
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_production_number_probe.py /private/tmp/typaxis-number-probe/number.pdf \
  /private/tmp/typaxis-number-probe/verified
```

The 1-page diagnostic probe has six source-order groups. Poppler and MuPDF both
extract both formula speeches, the surrounding body and `ABAB` exactly once
(disregarding extractor-specific whitespace). Independent pypdf traversal checks
the number Span, Formula parent, MCR and ParentTree. MuPDF renders the original
and a counterfactual with only the number MCID removed; the visible ink difference
has pixel bounds `[207, 67, 259, 84]` at 144 dpi. The image was inspected. The font
is a synthetic fixture whose A glyph is triangular, and body fixture outlines
are not a publication-quality face. These checks establish actual number paint
and extraction, not real-book typographic appearance or Harano acceptance.

The public writer/manifest, generic convergence and final line reshaping,
formal VMB exporter, 5,000 actual distinct engine formulas/mixed PNG, unchanged
Harano CID-CFF/IVS and full-book independent acceptance remain required.

Broader local regression:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-shaping -p typaxis-layout -p typaxis-pagination \
  -p typaxis-display-list -p typaxis-resources -p typaxis-cli \
  --lib --bins --locked -- --skip places_5000 --skip more_than_65535
```

Result: CLI **244 passed, 3 ignored, 3 filtered**; display-list **56 passed**;
layout **65 passed**; pagination **92 passed**; resources **28 passed**;
shaping **24 passed** — **509 passed**, no failures.
Log: `/private/tmp/typaxis-number-regression.log`. The final metric-sign guard is
covered by rerunning the three focused number tests with regenerated PDF output;
the guard rejects invalid horizontal ascender/descender signs just as body text
preparation does. No previously filtered large-corpus test or ignored external
validator is claimed as rerun here.

## 2026-09-07: 5,000 actual engine expressions and public scale admission

The preceding equation-number implementation changed code and passed regression
and independent probes, so it was progress. This turn generates the required
actual numeric expressions rather than relying on the earlier paint-color
cardinality corpus. The full objective and all public/full-book gates remain.

VMB commit `76b63a1a` adds `-corpus distinct-5000` to the existing fixture generator.
It renders `x_{i}=\frac{i}{i+1}` for i=1..5000 at 11pt, alternating inline/block,
with numeric template speech and genuine engine artifact identities. SVG files
are streamed; the index is written only after all rendering and engine Close
succeed. Existing outputs are refused. Both Go test packages passed. Generation
finished in 14.19 seconds with maximum RSS 184,041,472 bytes (not Typaxis runtime).

Corpus path: `/private/tmp/vmb-typaxis-distinct-5000-20260907`.
Index SHA-256: `486ae77ec68cc1ea462f0e107a44c48fcb9f6fd564ac08f1aed35a9f227f65af`.
Independent Python checks found 5,000 distinct raw hashes, derived hashes and
path geometry hashes, 71,682 paths and 1,521,172 source geometry segments. Those
segment counts are not admission replay/work charges. The checker verifies the
existing six-decimal tie-to-even export rule; an initial exact-rational comparison
was incorrect because the SVG serializes decimal6, and no exporter tolerance or
rounding rule was relaxed.

`tools/prepare_vmb_distinct_probe.py` creates a test-authored multi-container
package with exact source spans and per-occurrence semantics. It is not the
formal RenderBook exporter. It creates all 5,000 declarations and places every
one; alias mode places 8,000 occurrences from 5,000 declarations/100 contents.
Mixed mode has expressions 1..4952 and 48 distinct 2x2 RGB PNGs. Pillow independently
decoded all 48 images and verified distinct pixels.

Public CLI arguments (each job contains document-package.json and resources):

```sh
/private/tmp/typaxis-vmb-book-build/debug/typaxis check-package JOB/document-package.json \
  --package-root JOB --resource-root JOB \
  --profile typaxis.machine-pdf/production-book-1 --emit-diagnostics JOB/check-diagnostics.json
/private/tmp/typaxis-vmb-book-build/debug/typaxis build-package JOB/document-package.json \
  -o JOB/book.pdf --package-root JOB --resource-root JOB \
  --profile typaxis.machine-pdf/production-book-1 --no-compress \
  --emit-build-manifest JOB/build-manifest.json --emit-diagnostics JOB/build-diagnostics.json
```

No image/vector limit override or config override was supplied. Public check
results measured with `/usr/bin/time -l` outside the sandbox:

| Job suffix under `/private/tmp/typaxis-real-` | Declarations / occurrences | check result | seconds | maximum RSS bytes |
| --- | --- | --- | --- | --- |
| `distinct-5000-v2` | 5,000 / 5,000 | exit 0, empty diagnostics | 129.72 | 644,939,776 |
| `mixed-5000-v2` | 5,000 / 5,000 | exit 0, empty diagnostics | 123.20 | 637,435,904 |
| `alias-5000-v2` | 5,000 / 8,000 | exit 0, empty diagnostics | 151.38 | 606,470,144 |

Final package SHA-256 values, respectively:

- `ed0194b9895697bfc5685fe26955f778294766c8624059d01380ac6d06fb8ccd`
- `b234766f60cb608bd2fe7f89e1ec6ec30edf8f9a81ed35f8065e505b9b282874`
- `cf76265b0a124b068f6e7ef66a6a87b1b95cb87b633f81b0be6dcc4ad7f1f4c0`

Distinct check's successful log/diagnostics use `check2.log` and
`check2-diagnostics.json`. Earlier attempts are retained separately: the first
prototype omitted semantic_container anchor_id; v2 initially omitted the explicit
resource-root and failed opening font 0. The final invocations above correct
those authoring/host conditions without changing admission rules.

Distinct public build **failed** with `L5100: inline vector 3 exceeds an empty
line`, taking 194.78 seconds and maximum RSS 1,034,027,008 bytes. Its manifest has
`status=failed`, and `book.pdf` does not exist. Mixed check overlapped this build;
these observations are exploratory per-process timings, not a 1k/2.5k/5k scaling
baseline or the full performance gate. Mixed/alias builds were not redundantly
run against the same known first-inline failure.

The minimal unchanged first-expression reproduction is stored at
`/private/tmp/typaxis-real-overhang-repro`. It fails identically. The actual raw
metrics are advance 1,918,708, viewport width 2,008,820 and origin_x -45,056, while
the body width is 28,689,280. The old atomic line fit rejects visual_left < 0,
independently of total available width. The public pipeline still uses that
staging inline path. Connecting real paragraph/frame visual bounds and the
shared production path remains necessary; do not rewrite this corpus to avoid
the failing inline geometry or relax frozen old tests.

Added a scale PDF verifier for source-order occurrence/Do/structure/ActualText,
per-declaration use, Form sharing/BBox and full Poppler/MuPDF extraction. It has
**not** passed against the requested scale PDF, because that PDF is not yet
produced. Three harness tests passed: unused declarations cannot pass its gate,
PNG pixels are independently distinct, and existing output is not overwritten.
Command:

```sh
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  -m unittest discover -s tools -p test_vmb_scale_probe.py -v
```

Every process started in this turn has completed. This is genuine scale admission
and an actionable public-build failure, not scale PDF success. The formal
exporter, unchanged Harano, shared public convergence/paint/manifest and full-book
acceptance requirements remain open.
