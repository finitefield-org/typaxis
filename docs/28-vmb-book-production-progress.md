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
| CID CFF /2, FD-aware evaluator, subset / PDF integration | CID structure, FD-bound Type2 and internal whole-sfnt /2 admission verified on unchanged original Harano; selected closure/cache and real dense-CID subset verified internally, including independent outlines/UVS/raster comparison; name-keyed /2, aggregate resource owner and public PDF integration pending (checkpoint below) |
| Vertical tables, cmap 14, IVS shaping/extraction | Internal /2 admission and actual linked shaping preserve every original UVS and source cluster through dense subset mapping; independent full-UVS subset/render checks passed; package selection, ToUnicode/ActualText and public extraction binding pending (checkpoint below) |
| Contract 1.5 / production-book-2 / resource-set 3 and capabilities | Pending; publish atomically only after gates |
| VMB exporter geometry / metrics / semantics / source mapping | Geometry lowering, source projection and production math-adapter→per-occurrence wire/resource/semantic binding implemented in VMB; a real prepared-example public check gate passed below. Full RenderBook traversal, raster integration and final package/sidecar publication remain pending |
| VMB runner, explicit font/layout, environment isolation | Pending |
| Production with no native math | Empty native authorization implemented and regression passed; PDF body-font independence is still pending |
| Shared body/math flow and selected text placement | Authored shaping and LTR body/SVG line placement, measured paragraph/block/caption/raster/list placement, shared body fonts, Forms and selected PDF contributions verified below. Page-end candidate costs are now connected to the internal body cursor; Selected-line LTR shaping now has an actual bounded shape/rebreak owner (checkpoint below); final bidi, cumulative allocation, generated references, remaining subflows and public convergence/terminal/paint/manifest connection remain pending. |
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

## 2026-09-07: actual selected-line shaping and bounded body feedback

The common production kernel already accounts for negative visual overhang by
shifting glyphs and vectors together. The public scale failure recorded above
belongs to the frozen old atomic kernel. No frozen expectation was relaxed, and
no additional origin adjustment was added to the common kernel.

Added genuine selected-line context derivation, owner/UTF-8/grapheme partition
validation, and a linked-backend reshape API that clips pre/post context and
splits runs at those actual boundaries. Initial shaping retains its existing
fingerprint. Selected context contributes `typaxis.production-line-context/1`
to the reshaped receipt; it is not an independently supplied stability claim.

The new layout owner runs initial shaping and actual body-frame selection,
consumes the existing one-shot `LineReshapeFeedback` permits before each reshape,
and compares the complete selected-layout identity after each rebreak. A borrowed
`ProductionConvergedBodyLines` reaches the caller only after a stable comparison.
One candidate-work budget spans initial selection and all passes. The generic
feedback record reserve is now fallible before a permit is returned.

Authored test font `vmb-book/reshape/context.ttf` uses real OpenType calt to change
A into wider C before space B. Initial full-context shape emits GID 4; constrained
line context emits GID 2. Real width changes produce both stable two-pass output
and a one-line/two-line oscillation that stops at the configured pass limit.
The font was regenerated using Python 3.12 / FontTools 4.51.0 and its checked-in
SHA-256 reproduced exactly:
`abd89ef5ab470c02abf50090b7063114eae384387c36f339ae2cb85bfd7500d0`.
Its README records authored provenance, licenses and deterministic generation.

Focused command:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli --bin typaxis production_final_line_ --locked
```

Result: **7 passed**, no failures. In addition to genuine glyph/advance changes,
these check rejected foreign/incomplete/nonmonotone contexts, combining-grapheme
splits, exact object/hard/soft-break offsets, no consumer call on oscillation,
and exact shared candidate budget N acceptance / N-1 rejection. An initial
integration test omitted the paragraph's real horizontal indents from its body
width; the test now derives those indents from the resolved block style. Product
frame rules and font metrics were not changed to accommodate the test.

This does not close final bidi L1/visual line selection, allocation accounting
across all pipeline stages, page/generated-reference convergence, public writer
receipt/manifest binding, or unchanged full-book/Harano gates. The existing
per-stage fragment ceilings and finite reshape quota are retained. This owner
is not yet the public book pipeline; the old public overhang failure remains a
required connection task, not a passing PDF gate.

Broader verification:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-cli -p typaxis-shaping -p typaxis-linebreak -p typaxis-layout \
  -p typaxis-pagination -p typaxis-display-list -p typaxis-resources \
  --lib --bins --locked -- --skip places_5000 --skip more_than_65535
```

Result: CLI **251 passed, 3 ignored, 3 filtered**; display-list **56 passed**;
layout **65 passed**; linebreak **44 passed**; pagination **92 passed**;
resources **28 passed**; shaping **24 passed** — **560 passed**, no failures.
The previously recorded large placement probes were not rerun by this command,
and the three ignored independent-PDF probes are not counted as passed here.
Logs: `/private/tmp/typaxis-reshape-tests.log` and
`/private/tmp/typaxis-reshape-regression.log`. All verification started for this
checkpoint reached terminal states. No branch push or public PDF was performed.

## 2026-09-07: original CID CFF program structure for /2

Added `typaxis-font/src/cff_v2.rs` as a separate structural inspection stage.
It retains one `Arc<[u8]>` of the unchanged CFF table and stores verified ranges
for CharStrings/global/local programs. The existing CFF /1 admission, program,
evaluator and resource registry have not been widened.

The parser validates CID Top DICT/ROS, FontMatrix constraints, FDArray 1–256,
FDSelect 0/3, CID charset 0/1/2, each FD's Private DICT and local INDEX. It checks
INDEX counts before allocation and rejects the 49th DICT operand before retaining
it. Global and all FD-local subroutine declarations share one checked limit.
Identical Private/local INDEX ranges may be shared across FDs, with repeated
per-FD charges; partial overlaps and aliases across different structure kinds
are rejected. The result is explicitly an inspection, not font admission or an
embedding/evaluation/subset/PDF receipt.

Independent original-font command:

```sh
python3 tools/inspect_harano_cff_program.py \
  /Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf
```

The tool requires the recorded original SHA-256 and uses FontTools 4.51.0 here.
It reports 6,422,896 original bytes, 23,060 glyphs, 18 declared FDs, 1,600 global
and 24,956 local subroutines (26,556 total). FD 12 contains 21,626 local Subrs.
The complete dense FD map SHA-256 is
`83ca997d6773a631791dad08bf2448ed15566a7f51b052d7776a8a364cc344e1`;
the complete GID→CID map serialized as big-endian u16 hashes to
`22a2721ffa80fc5fe1da53fe8a78f0a4a0c59a4f73a32ee62f30177f25ce04aa`.
These independently obtained expectations are asserted by the Rust original-font
probe. All 18 FDs are parsed and charged even though only 12 distinct FDs have
referencing glyphs. The first probe incorrectly expected every declared FD to be
used; FontTools confirmed the actual use set and full mapping hashes before that
test expectation was corrected. No product input or parser rule was relaxed.

```sh
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font cff_v2_original_harano_program --locked -- --ignored --nocapture
```

Result: **1 passed**. The test checked the original full-file SHA-256, complete
FD/CID maps, all charstring ranges, glyph/FD counts and selected FD width defaults.
This test extracts the original table in memory; it does not strip or rewrite
the font. Output: `/private/tmp/typaxis-cff-v2-harano.log`; independent facts:
`/private/tmp/typaxis-harano-cff-program-facts.json`.

Broader regression command:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font -p typaxis-resources -p typaxis-resource-admission \
  -p typaxis-shaping --lib --locked
```

Result: font **29 passed, 1 ignored**; resource-admission **62 passed**;
resources **28 passed**; shaping **24 passed** — **143 passed**, no failures.
The single original-font test is explicitly run separately above. A subsequent
additional regression verifies that a FDArray cannot also serve as a CharStrings
INDEX; it changes tests only. Final `-p typaxis-font cff_v2 --locked` result:
**9 passed, 1 ignored**, no failures. Tests also cover count N/N+1, shared-local
subroutine accounting, invalid FDSelect coverage/sentinel/FD values, CID
range overflow/duplicates, partial Private overlap and FD matrix rejection.

Full /2 sfnt admission, vertical tables/cmap14/IVS, FD-aware Type2 execution and
width/hmtx consistency, selected-glyph subset/PDF integration, versioned registry
publication and original full-book rendering/extraction remain mandatory pending
work. Structural inspection success does not fulfill the Harano support gate.

## 2026-09-07: FD-bound Type2 execution, all original Harano outlines

Added a separate /2 inspection-work session and immutable glyph FD context.
Global→local calls retain that glyph's FD, with local/global biases derived from
the correct INDEX counts. The existing Type2 path, curve, flex, mask and budget
mechanics are shared through private access/budget traits. V1 retains its original
program, width policy, terminal policy, receipts and algorithm identities.
V2 diagnostics retain GID, FD, original table offset and single/escaped operator.

Two actual-input findings corrected the initial implementation:

- Original .notdef ends in a global subroutine. Type2 permits endchar inside a
  subroutine (Adobe Type2 §4.2 Note 6). /2 now finishes the glyph in that case;
  the frozen /1 restriction is unchanged. A nested global→local→endchar test
  exercises the new policy directly.
- The unchanged original has 310 glyphs whose CFF/PostScript width differs from
  hmtx, including GID 151 (346 versus 1000). FontTools independently confirms
  these differences. OpenType explicitly uses hmtx advance rather than CFF width.
  Design §7.4/7.7 now states this correctly: validate and preserve CFF source
  width arithmetic, use admitted hmtx for shaping and output widths, and produce
  consistent subset CFF/hmtx/PDF widths from hmtx. Original-value equality is not
  an admission condition. No source font was altered and no layout advance was
  changed to the CFF width. The inspection result retains both widths.

Independent execution:

```sh
python3 tools/inspect_harano_cff_outlines.py \
  /Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf
```

The tool checks the unchanged full-file SHA-256, executes each original
CharString through FontTools 4.51.0's RecordingPen, records Move/Line/Cubic/Close
and signed 16.16 coordinates, and incorporates GID/hmtx advance into a chained
SHA-256. It separately hashes every decoded CFF source width. It does not subset
or rewrite the font. Output: `/private/tmp/typaxis-harano-outline-facts.json`.

All **23,060 glyphs** passed the Rust evaluator under one **default** work budget:
**8,376,159 operations** and **1,572,638 outline segments**. No operation/outline
limit override was used. The complete outline/advance state hash equals the
independent result:
`f3a7b806eb38a37c56ec76eac5b80dd21a1bfe1f17a71650c971b3c399bace38`.
The complete decoded source-width hash also matches:
`feb4b1cdc05ab3b6ef6b3a8ead1167be85068ecac3cdfda03cc7f8d97f39113d`.
The Rust test asserts both hashes and the 310 distinct-width count. This covers
all original outlines, not only selected Latin or sample Japanese glyphs.

Broader verification:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font -p typaxis-resource-admission -p typaxis-resources \
  -p typaxis-shaping --lib --locked
```

Result: font **37 passed, 2 ignored**; resource-admission **62 passed**;
resources **28 passed**; shaping **24 passed** — **151 passed**, no failures.
The seven new execution tests cover cross-FD global/local selection, distinct
PostScript/OpenType widths, width overflow location, cumulative exact N/N-1
operation/segment budgets, nested endchar, invalid local index, missing mask,
operand overflow, escaped-op diagnostics and recursive-call depth. Original /1
admission/subset/budget and detailed-diagnostic regressions passed in this run.
Log: `/private/tmp/typaxis-cff-execution-regression.log`.

The two explicit original-font tests are run separately on the final code:

```sh
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font cff_v2 --locked -- --ignored --nocapture
```

This is still not the public Harano gate. The next /2 owners must connect sfnt
admission/vertical/cmap14/IVS, source/face/profile-bound selected-GID caching and
subset receipts, then PDF/manifest publication and the unchanged full-book
render/extract tests. The all-glyph execution above is a test; production still
must evaluate only selected glyphs. No contract/profile registry was enabled.

Final original-font run: **2 passed**, no failures, including the complete
independent outline/width hash assertions. Log:
`/private/tmp/typaxis-cff-execution-harano-final.log`. Verification started for
this checkpoint reached terminal states; no branch push or public PDF occurred.


## 2026-09-07 checkpoint: bounded vertical tables and variation coverage

Added separate CFF /2 table validators. They borrow unchanged input bytes and
provide sparse VORG/default origins, long/trailing vmtx metrics, and compressed
format-14 Default/NonDefault(GID)/Missing lookup. No vertical value is applied to
horizontal positioning. vhea accepts versions 0x10000 and 0x11000, checks reserved
fields and requires its paired vmtx with exact glyph-derived length.

UVS validates platform/encoding identity, selector order, scalar/range order,
GID range, default/non-default disjointness and payload overlap. Exact same-kind
payload sharing is allowed and charged per selector. Fixed limits are 256
selectors and 1,000,000 expanded-equivalent values, checked without expansion.
Errors retain table-relative byte offsets and explicit limit/observed fields.
This supplements, rather than completes, base cmap validation and IVS shaping.

Independent evidence (FontTools 4.51.0, unchanged source SHA-256
`66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717`):

```sh
python3 tools/inspect_harano_cff_tables.py \
  /Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf
```

The tool checks every GID in order and serializes origin i16, advance u16 and
bearing i16, all big-endian. All 23,060 records hash to
`a48a869c98cf13ff94cd763ccabf2dbdf849bedc531ea46126a36a075db6c676`.
There are 21,012 long metrics and 152 VORG overrides. All 17 selectors and 14,780
UVS values, ordered by selector/base and serialized as two big-endian u32 values,
a default/non-default byte and big-endian u16 GID (zero for default), hash to
`ed20f9a7d92d9331403ef147385f2025670839d3af63f3af8e91265567c789a8`.
Rust tests assert both independently obtained hashes using the unchanged source.

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font -p typaxis-resource-admission -p typaxis-resources \
  -p typaxis-shaping --lib --locked
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font cff_v2_ --locked -- --ignored
```

Regression result: **162 passed** (font 48, resource-admission 62, resources 28,
shaping 24), no failures; four explicit-original tests ignored in this run.
Separate original-font result: **4 passed**, no failures, including existing
all-glyph Type2 and CID/FD structure tests. Logs:
`/private/tmp/typaxis-cff-tables-regression.log` and
`/private/tmp/typaxis-cff-tables-harano.log`.
Boundary tests cover every truncated prefix, trailing lengths, missing table
pairs, wrong versions/reserved values, glyph order/range, invalid encoding,
Unicode surrogate crossings, UVS category intersections, absent offsets,
selector/value limits and charged shared payloads. Public /1 behavior and
profile/contract registry remain unchanged. No public Harano/full-book gate or
IVS shaping/extraction success is claimed; those remain required work.


## 2026-09-07 checkpoint: original CID sfnt admission and resolved UVS coverage

`admit_sfnt_cff1_v2` now returns a private-field `Cff1AdmissionV2` owning the exact
original source, source hash, glyph/horizontal metrics, base+UVS coverage and
validated CID program. It checks standalone face 0, source-byte ceiling, the
shared sfnt directory/order/range/checksum logic, required and existing optional
tables, embedding permission, vertical tables, combined cmap and program.
CFF name/FontBBox must match sfnt PostScript name/head bbox; this is checked in
the same program pass, without reparsing with infallible allocation assumptions.
The identity includes source hash, face 0, effective-limits fingerprint and the
/2 admission/resource-profile identifiers. Admission does not execute glyphs.
The old /1 entry point still rejects the unchanged original font at VORG.

The cmap owner validates format 14 first, then uses the same format 4/12 base
validation and cross-map consistency checks as /1, with the supplemental record
excluded from the base map. Coverage does not drop selectors. Non-default
mappings may have no base-cmap entry; default entries require one at lookup time.
Unsupported pairs, isolated selectors and GID 0 do not produce covered glyphs.
Actual two-scalar shaping, source-cluster preservation and PDF extraction remain
required; this owner alone does not establish those properties.

Independent `tools/inspect_harano_cff_tables.py` now also hashes every base
mapping (big-endian scalar u32/GID u16) and every resolved UVS (selector u32,
base u32, GID u16). The unchanged original has **15,815 base mappings**, hash
`95df78a2d881b8387c208f4f9540bf86a292855a0a6beb4d1b7fa17fad2baecf`;
all **14,780 resolved UVS** hash to
`825b70e8ec61dc7e6b9b6909910cdbb30a23e25f203b1f1f7b1aa2c429132214`.
The Rust test asserts both independently obtained hashes.

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font -p typaxis-resource-admission -p typaxis-resources \
  -p typaxis-shaping --lib --locked
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font cff_v2_ --locked -- --ignored
```

Regression: **166 passed** (font 52, resource-admission 62, resources 28, shaping
24), no failures, six explicit-original tests ignored. Log:
`/private/tmp/typaxis-cff-sfnt-regression.log`.
Whole-sfnt original test also checks deterministic admission identity, authoritative
hmtx for the known differing-width glyphs, /1 rejection, exact maxp lower-limit
diagnostics, and checksum-correct negative copies for restricted fsType,
VORG version and vhea reserved fields. These copies are negative cases only.
No original bytes were edited to obtain positive admission.

This is internal CID admission, not public Harano support. The resource registry,
name-keyed /2 path, source/face/profile-bound selected cache and closure, subset,
shaping/ToUnicode, PDF/manifest and all full-book gates remain required. No public
profile or contract registry was enabled and no public scale/full-book PDF was
produced in this checkpoint.

Final explicit-original run: **6 passed**, no failures. Log:
`/private/tmp/typaxis-cff-sfnt-harano-final.log`. All verification processes for
this checkpoint reached terminal states; no public PDF or branch push occurred.


## 2026-09-07 checkpoint: source-bound selected CFF evaluation and closure

Added `Cff1GlyphClosureV2` and `Cff1SubsetSessionV2`. Selection validates the
complete GID set and the existing nonzero-CID count ceiling before execution,
adds .notdef once and assigns dense subset GIDs in ascending original GID order.
The sealed closure identity binds admission, source, face ID, instance ID and
selection. A closure from another source or effective budget cannot be prepared.

The session shares a single Type2 operation/outline budget. The fixed profile
is /2 and standalone source face index is 0; variable cache keys are source
SHA-256/GID. Identical source aliases across font instances and face IDs reuse
work. Each actual evaluation reads advance from the admission's hmtx, never from
caller-supplied width or the CFF PostScript width. Invalid requested GIDs fail
before any evaluation. Successful results are cached; failed work is not reset.

The explicit-original test selects GIDs 0, 151 and 233 from unchanged Harano.
It proves dense mapping, exact N/N-1 selected-count limits, no execution for an
invalid set, no evaluation of an unselected GID, repeated/alias cache reuse,
correct hmtx for the known CFF-width disagreements, foreign closure/budget
rejection and exact operation/segment N/N-1 boundaries. A separate checksum-correct
source with allowed PreviewAndPrint fsType and otherwise identical outlines
proves source isolation: its original closure is rejected and preparing its own
closure doubles work and cache entries. This source-isolation copy is not used
as the unchanged-original support gate.

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font -p typaxis-resource-admission -p typaxis-resources \
  -p typaxis-shaping --lib --locked
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font cff_v2_ --locked -- --ignored
```

Regression: **166 passed** (font 52, resource-admission 62, resources 28, shaping
24), no failures; seven original-font tests ignored in the regular run. Logs:
`/private/tmp/typaxis-cff-selection-regression.log` and the focused original test
`/private/tmp/typaxis-cff-selection-harano.log` (one passed).

The next step remains real subset bytes/receipt generation. This session has not
yet generated an OpenType subset or authorized a PDF. Aggregate owners still
must close all instance selections, prepare per-face unions in FontFaceId order,
then bind subset/ToUnicode/manifest output. IVS shaping, public profile activation,
all full-book gates and the remaining common-layout/public-writer work are still
mandatory. No public profile registry was enabled in this checkpoint.

Final explicit-original verification: **7 passed**, no failures. Log:
`/private/tmp/typaxis-cff-selection-original-final.log`. All checks launched for
this checkpoint reached terminal states. No public PDF or branch push occurred.


## 2026-09-07 checkpoint: real selected OpenType subset, UVS and raster equivalence

`Cff1SubsetSessionV2::subset` now returns `Cff1SubsetV2` with actual OpenType
bytes, source/dense mapping, authoritative widths, PDF metrics and a /2 receipt
bound to source and sealed closure. Shared canonical Type2 and sfnt/header/name/
horizontal/PDF-metric recipes preserve /1 behavior. Source CFF/PostScript width
is not copied into output widths: CFF, hmtx and future PDF Widths use admitted hmtx.
CharString-byte subtotal and final sfnt size are bounded; the final sfnt buffer
is allocated after its exact size check. Complete cumulative accounting of all
temporary serializer/pipeline allocations remains a separate mandatory item.

The subset retains selected base cmap entries and all resolvable selected UVS.
Source defaults and non-defaults both become explicit dense-GID non-default
mappings in the canonical format 14. UVS-only selection works with an empty
format 12; input validation still requires a base subtable and, for an empty
base map, a usable non-default UVS. The ordinary regression test covers this
case and rejects an empty-base table containing only unusable/default coverage.

The original-font fixture selects **28 glyphs**, including Japanese text,
punctuation, UVS, all **12 actually used FDs**, and the two known CFF/hmtx width
disagreements. It produces **5,052 bytes** with SHA-256
`6557c69c765076c5872ce68e4e49ff91eaa1119d6fac36df4abfdf82231765ff`.
The Rust test asserts deterministic bytes/receipt, no repeated Type2 work,
selected hmtx/Widths, all retained/dropped original UVS keys, sfnt checksums and
exact 5,052/5,051-byte success/failure. It does not pass a generated subset off
as an original input: source admission deliberately rejects subset-prefix names.

```sh
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
TYPAXIS_HARANO_SUBSET_OUTPUT=/private/tmp/typaxis-harano-selected-v2.otf \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font cff_v2_ --locked -- --ignored
python3 tools/verify_harano_cff_subset.py \
  /Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  /private/tmp/typaxis-harano-selected-v2.otf \
  /private/tmp/typaxis-harano-selected-v2.otf.gids
```

FontTools 4.51.0 independently compares every selected Type2 outline command and
fixed coordinate, source/output advance and bearing, output CFF width, dense CID,
**27 base mappings and 12 UVS pairs**. All match. Output uses one FD, no local or
global Subrs. The selected outline/mapping hash is
`6d9824bced9f9d8cd9ab905fa17e36cef5b3ac663b9eeb626ac90b630aed913f`.
Facts: `/private/tmp/typaxis-cff-subset-fonttools-facts.json`. FontTools warns
about the deliberately zeroed deterministic head timestamps; this does not
alter bytes or the comparison. Original source hash is checked before parsing.

Independent raster verifier (Homebrew dependency flags were obtained through
`pkg-config --cflags --libs freetype2`):

```sh
cc -Wall -Wextra -Werror \
  -I/opt/homebrew/opt/freetype/include/freetype2 \
  -I/opt/homebrew/opt/libpng/include/libpng16 \
  -L/opt/homebrew/opt/freetype/lib tools/verify_cff_subset_raster.c \
  -lfreetype -o /private/tmp/typaxis-verify-cff-subset-raster
/private/tmp/typaxis-verify-cff-subset-raster \
  /Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  /private/tmp/typaxis-harano-selected-v2.otf \
  /private/tmp/typaxis-harano-selected-v2.otf.gids
```

FreeType **2.14.3** rendered all 28 source/dense GID pairs at 12, 24, 48 and 96px:
**112 exact bitmap, origin and advance comparisons passed**. Both fonts use
NO_HINTING/NO_AUTOHINT/NO_BITMAP to compare preserved outlines; canonical CFF
subsets do not copy hint programs. This is not a default-hinted comparison or a
full-book PDF rendering claim. Log:
`/private/tmp/typaxis-cff-subset-freetype-final.log`.

Final explicit-original run: **8 passed**, no failures, log
`/private/tmp/typaxis-cff-subset-original-final.log`. Final ordinary regression:
**167 passed** (font 53, resource-admission 62, resources 28, shaping 24), eight
original tests ignored, no failures:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font -p typaxis-resource-admission -p typaxis-resources \
  -p typaxis-shaping --lib --locked
```

Log: `/private/tmp/typaxis-cff-subset-regression-final.log`. All launched processes
reached terminal states. Actual IVS shaping/source clusters/ToUnicode, public
resource/PDF/manifest binding, name-keyed /2 and all full-book/scale/platform gates
remain mandatory. No public profile registry, public PDF or branch push occurred.


## 2026-09-07 checkpoint: actual linked shaping of every original UVS

`shape_cff1_run_v2` supplies only the sealed admission's original bytes, face,
metrics and effective limits to the existing `shape_linked`/harfrust backend.
It preflights complete base+VS pairs, refuses isolated/missing/split selectors,
retains the original UTF-8 and exposes exact logical-cluster text. No selector
is dropped or replaced. Input plus pre/post context obey the admission's context
byte ceiling; the existing backend record-bound/reservation/output accounting
is reused. This is a font-level integration, not package style selection or
layout/PDF authorization. Old /1 default-ignorable behavior remains unchanged.

The original-font test shapes **all 14,780 UVS**, in bounded runs of 200 pairs
separated by ordinary spaces. Every actual backend GID, UTF-8 source range and
`cluster_text` equals the expected original pair. It explicitly covers distinct
sequences sharing one GID. The actual selected output union generates a
**14,674-glyph**, **6,563,684-byte** subset with SHA-256
`aad51459429ea109f404e42b29e5b38e350430d9bdebbefbaff10c328b1d481c`.
All original pairs resolve through the resulting dense subset mapping. Default
Type2 work usage is **7,422,048 operations**, **1,354,039 segments**, with no
limit overrides. The test also refuses a missing pair with exact relative byte
range 3..8, standalone VS, repeated VS, post-context splitting, excessive context
and mismatched source-span length.

```sh
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
TYPAXIS_HARANO_IVS_SUBSET_OUTPUT=/private/tmp/typaxis-harano-all-ivs-v2.otf \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-shaping cff_v2_shape_original --locked -- --ignored --nocapture
python3 tools/verify_harano_cff_subset.py \
  /Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  /private/tmp/typaxis-harano-all-ivs-v2.otf \
  /private/tmp/typaxis-harano-all-ivs-v2.otf.gids
/private/tmp/typaxis-verify-cff-subset-raster \
  /Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  /private/tmp/typaxis-harano-all-ivs-v2.otf \
  /private/tmp/typaxis-harano-all-ivs-v2.otf.gids
```

The raster binary is built from the C verifier with the dependency flags in the
previous checkpoint. FontTools 4.51.0 verifies all **14,674 outlines**, source/
output hmtx and normalized CFF widths, **14,002 base mappings** and **14,780 UVS**.
Selected outline/mapping hash:
`127b543d1f072edb1af4e404c9cacc6544be0485f50bb903b60a64c10bc4108b`.
This set uses source FDs **3, 5, 12, 14** and has no CFF/hmtx disagreements; it
does not replace the previous all-12-FD/width-disagreement fixture.
FreeType 2.14.3 passes **58,696** exact unhinted bitmap/origin/advance comparisons
at 12/24/48/96px. Facts/logs:
`/private/tmp/typaxis-harano-all-ivs-fonttools.json`,
`/private/tmp/typaxis-harano-all-ivs-freetype.log`,
`/private/tmp/typaxis-cff-shape-original.log`.

Final ordinary regression: **167 passed** (font 53, admission 62, resources 28,
shaping 24), ten explicit-original tests ignored, no failures. Log:
`/private/tmp/typaxis-cff-shape-regression.log`.

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font -p typaxis-resource-admission -p typaxis-resources \
  -p typaxis-shaping --lib --locked
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font -p typaxis-shaping cff_v2 --locked -- --ignored
```

The font-level all-UVS result is not a ToUnicode/ActualText or public full-book
extraction claim. Package selection and the exhaustive profile-specific resource,
shaping, PDF and manifest owners still need private-staging integration under
§9.1, followed by the public/all-book gates before publication. Full exporter,
common-layout closure and scale/platform verification remain mandatory.

Final explicit-original verification: **10 passed** (font 8, shaping 2), no
failures. Log: `/private/tmp/typaxis-cff-shape-original-final.log`. Every process
launched for this checkpoint reached a terminal state. No public profile
registry, public PDF, or branch push occurred.

### CFF /2 sealed font plans and direct PDF objects (2026-09-07)

Added `FrozenPdfCff1PlanV2` independently of the /1 encoder receipt. The owner
closes every instance before Type2 evaluation, evaluates sorted face unions in
one session, and emits instance-sorted plans. It retains original cluster text,
source provenance and explicit ActualText requirements. Parsed buffer/range and
generated owner/kind/ordinal participate in plan identity. Conservative escaped
JSON capacity is charged before serialization. This is local font accounting;
the cumulative document allocation owner remains necessary.

The PDF consumer writes six objects directly from the sealed /2 plan, preserving
FontFile3/OpenType, CIDFontType0, dense widths and CIDSet. Existing ToUnicode and
CIDSet serialization helpers are shared with /1 without converting new plans
into old receipts. Output contains offsets and the source plan fingerprint;
object reservation/collision checks and publication belong to the document owner.

Validation:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-font -p typaxis-shaping -p typaxis-resources -p typaxis-pdf --locked
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-resources cff_v2 --locked
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
TYPAXIS_CFF_V2_PDF_OUTPUT=/private/tmp/typaxis-cff-v2-font-objects.pdf \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-resources -p typaxis-pdf cff_v2 --locked -- --ignored
pdftotext -enc UTF-8 -raw /private/tmp/typaxis-cff-v2-font-objects.pdf \
  /private/tmp/typaxis-cff-v2-font-objects-extracted.txt
pdffonts /private/tmp/typaxis-cff-v2-font-objects.pdf
mutool draw -q -F txt -o /private/tmp/typaxis-cff-v2-font-objects-mupdf.txt \
  /private/tmp/typaxis-cff-v2-font-objects.pdf
```

Ordinary regression: **183 unit tests + 2 doc tests passed**, 12 explicit-original
tests ignored. The subsequently added generated-provenance test passes in the
**3-test** targeted resource run (two overlap the regression). Original-font
resource and PDF tests: **2 passed**. Tests cover source identity, generated
namespace identity, same-GID base/IVS extraction, deterministic bytes, shared-face
instances, duplicate/mismatched inputs, object range and limits identity.

Poppler **26.08.0** and MuPDF **1.28.2** both extract exactly `A一一󠄀日本語`
after removing only trailing reader-added newline/form-feed characters, compared
with the original `.pdf.txt` artifact. Poppler reports `CID Type 0C (OT)`,
Identity-H, embedded/subset/Unicode all yes. The PDF is a diagnostic one-page
assembly of the real font contribution, **not** a public PDF receipt or full-book
evidence. Logs: `/private/tmp/typaxis-cff-v2-pdf-regression.log`,
`/private/tmp/typaxis-cff-pdf-plan-generated-tests.log`,
`/private/tmp/typaxis-cff-v2-pdf-original-final.log`.

Public profile/manifest union, package font selection, selected-layout paint
closure, formal exporter, full-book/scale and both-host gates remain mandatory.
No public profile was activated by this checkpoint.

### Private resource-set /3 host admission and font instances (2026-09-07)

Added a separate staging resolver and immutable /3 ledger with exhaustive
TrueType/CFF-v2 font variants. Stable reads, host session checks, aggregate
resource bytes, image parsers and vector work budgets use the existing resolver.
CFF /2 is held in a separate map, validates declaration/media/hash/face and can
only finish into the new ledger. No old CFF receipt or old resource ledger is
constructed for it. CFF failures retain the font-face subject and typed /2 detail.
The ledger fingerprint binds /3 identity, source/declaration, CFF admission,
limits and the supplied upstream profile fingerprint. Runtime session identity
is retained independently of content fingerprint. Canonical JSON capacity is
bounded before allocation; complete pipeline allocation accounting is still due.

The new font-instance table borrows the actual ledger and resolves dense IDs in
selected-face order without accepting a substitute ledger. Style selection and
actual shaping/paint/manifest connections to this owner remain outstanding.

Verification:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-resource-admission --locked
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-resource-admission production_v3 --locked -- --ignored
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-shaping -p typaxis-resources --locked
```

Admission: **64 unit tests and 2 compile-fail doc tests passed**, one original
font test ignored. Explicit original: **1 passed**, using unchanged Harano plus
TrueType and PNG through real host reads. Two Harano declarations preserve the
original bytes/admission and distinct face IDs. Different completion orders
produce identical content fingerprints; distinct runtime sessions remain distinct.
The old resolver still rejects Harano. Other new tests reject foreign pending
sessions, duplicate binding, missing resources and out-of-range selected faces.
An exact combined font/image byte ceiling succeeds; one byte less fails on image
read and cannot be bypassed by retry. Logs:
`/private/tmp/typaxis-production-v3-admission-regression.log`,
`/private/tmp/typaxis-production-v3-original.log`,
`/private/tmp/typaxis-production-v3-downstream-regression.log`.

No public contract/profile/schema alias was changed. The upstream typed 1.5
preflight/manifest union and instance-to-shaping connection are still required,
along with formal exporter, common layout, public PDF and full-book/scale/host gates.

### Ledger-bound /3 shaping and multi-size CFF sharing (2026-09-07)

Connected ledger-issued instances to actual linked shaping for TrueType and
CFF /2. The input has no caller-selected font ID or bytes: those come from the
sealed instance. The output retains that instance plus the exact request,
including source, language/script, bidi level, size and context. Feature behavior
remains the linked backend's fixed defaults.

Added an instance-table fingerprint over the selected face set and /3 ledger,
with bounded input iteration and canonical allocation. This distinguishes
numeric instance IDs reused by different selected tables. The CFF resource
bridge now derives face/admission from these shaped instances, requires the same
runtime session, ledger fingerprint and instance-table fingerprint, and keeps a
reference to the actual ledger. It rejects TrueType instead of silently omitting
it from a mixed document; the complete mixed-font finalizer is still required.

Found and corrected an incompatible restriction in the new CFF /2 font plan:
it had required one font size per instance. FontInstance does not include size,
and §14 requires body/caption/heading to share document font usage. Size now
belongs to each frozen cluster and its canonical identity. Different sizes of
the same face/GID share subset bytes and CID bindings.

Validation:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-resource-admission -p typaxis-shaping -p typaxis-resources -p typaxis-pdf --locked
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
  cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-resource-admission -p typaxis-resources -p typaxis-pdf --locked -- --ignored
cargo test --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build \
  -p typaxis-resources production_v3 --locked
```

Ordinary regression: **196 unit + 4 doc tests passed**, 6 original-font tests
ignored. Explicit originals: **4 passed** (admission 1, resources 2, PDF 1).
After retaining the full request in the shape owner, the targeted ordinary
shape test also passed. Real host-read Harano at 11pt and 22pt uses one shared
CID/subset, exact IVS source and distinct cluster sizes; actual advance doubles.
Foreign runtime sessions with equal content fingerprints and different selected
tables are rejected. TrueType shape, exact source, context bounds and source
length mismatch are tested. Logs:
`/private/tmp/typaxis-production-v3-shape-regression.log`,
`/private/tmp/typaxis-production-v3-shape-original-final.log`,
`/private/tmp/typaxis-production-v3-bound-input-tests.log`.

The upstream typed contract/profile receipt, full mixed-font finalizer, common
paragraph/page/paint ownership and public manifest/writer remain outstanding.
Formal exporter, real chapter/full-book/scale and both-host gates are still part
of the unchanged completion scope. No public profile or schema alias changed.

### RenderBook body traversal in the companion exporter (2026-09-07)

Companion VMB commit **`6fed376c`** adds `LowerBookBody` under
`vmb-core/internal/rendertypaxis/package_body.go`. It walks section headings,
paragraphs, strong/emphasis, breaks, external links, ordered/unordered lists and
actual inline/block math through one source projection. Node IDs, spans and text
buffers come from the traversal. Number prefixes are generated provenance;
number presentation and inline spacing are explicit inputs. Unhandled populated
fields and unsupported node kinds fail with source context and no partial result.
Allocation checks precede child-array reservation; projection enforces each real
node. Math adapter cleanup uses a separate bounded context after cancellation.

Ordinary Go tests passed for `rendertypaxis`, `render` and `rendermath`; the final
rendertypaxis run also passed with the exact-node-boundary and explicit-spacing
tests. Real source-bound VMB math is used in the traversal fixture. All wire
nodes match projection IDs/spans; text mappings, original math occurrences and
repeat-execution bytes are checked. Public `check-package` passed for both the
prior math test and the new body test. The new body has **2 images, 2 formula
occurrences and 88 projection bytes**.

```sh
cargo build --manifest-path workspace/Cargo.toml \
  --target-dir /private/tmp/typaxis-vmb-book-build -p typaxis-cli --locked
cd /Users/kazuyoshitoshiya/v/vmb-container/vmb-core
go test ./internal/rendertypaxis/... ./internal/render/... ./internal/rendermath/...
go test ./internal/rendertypaxis/... -count=1
VMB_TYPAXIS_CLI=/private/tmp/typaxis-vmb-book-build/debug/typaxis \
VMB_TYPAXIS_FIXTURE_ROOT=/Users/kazuyoshitoshiya/t/typaxis/samples/machine-package/profiles/production-book-1/combined/job \
  go test ./internal/rendertypaxis -run 'TestBookBodyPublicCheckPackage|TestMathExportPublicCheckPackage' -count=1 -v
```

Binary SHA-256: `2fe9e0cc5c233561ee3d29385e45b867c896d58ff6d753b383a1e5488f48677f`.
Body test package SHA-256:
`85e3a5ac0b185a48d70204593e9c3699b588a2ca867fc707507529f42a0ec329`.
Source SHA-256: `fe9f74e6f7aaa1bdb78945e828cd4866ad6801bb4bcc3780b25de65f5669a6e3`.
Logs: `/private/tmp/vmb-typaxis-body-regression.log`,
`/private/tmp/vmb-typaxis-body-final-tests.log`,
`/private/tmp/vmb-typaxis-body-public.log`.

This is a body contribution, not the complete exporter: the public test still
joins it to a known test envelope. Metadata/font/style/page/outline assembly,
remaining table/figure/reference/container/footnote traversal, bounded collection
of all findings, CLI adapter and ArtifactSink publication are outstanding. It
neither proves build/PDF success nor replaces the chapter, full-book, distinct/
alias scale and both-host gates. All launched processes reached terminal state;
no public profile was changed or branch pushed.

### 2026-09-07: VMB table and footnote body traversal

Companion VMB commits **`5f707a80`** (tables) and **`ef965c51`** (footnotes)
extend the actual `LowerBookBody` traversal. Tables receive explicit fixed/fraction
column sizing, normalize cells to declared column order, retain headers and use
the common body/math source projection inside cells. Unknown/duplicate/missing
columns, unresolved widths and unhandled fields are rejected with source context.
Right/center alignment still requires style binding; table captions/numbering
remain unimplemented and are rejected instead of dropped.

Footnote definitions encountered in sections, lists and table cells are queued,
ordered by the resolved VMB number and emitted after the body in wire preorder.
Definition IDs and contiguous numbering are validated, as are reference targets,
resolved numbers and supported presentation. Source text contains no provisional
marker digits. References and definitions have separate spans; inline math in a
footnote keeps its real source and receives the final footnote package pointer.
Unsupported nested notes/references and populated fields fail without partial
output. Child-array checks preserve parent source context.

Verification used original source-bound VMB math, reversed table cell input,
reversed footnote definition encounter order, list-contained definitions, exact
node ceilings and ceilings minus one. Eight table and eleven footnote negative
cases passed. All rendertypaxis tests, including four explicitly enabled public
`check-package` cases, passed in **22.150 seconds**. Prior body/math/table public
hashes remained identical after the footnote changes.

```sh
cd /Users/kazuyoshitoshiya/v/vmb-container/vmb-core
VMB_TYPAXIS_CLI=/private/tmp/typaxis-vmb-book-build/debug/typaxis \
VMB_TYPAXIS_FIXTURE_ROOT=/Users/kazuyoshitoshiya/t/typaxis/samples/machine-package/profiles/production-book-1/combined/job \
  go test ./internal/rendertypaxis/... -count=1 -v
```

Both new public cases have **1 image, 1 formula occurrence and 58 projection
bytes**, with empty diagnostics. Binary SHA-256 remains
`2fe9e0cc5c233561ee3d29385e45b867c896d58ff6d753b383a1e5488f48677f`.
Table package/source SHA-256:
`56ed751c26efe6fe89694d949fa766005785634a7dfce3288434da990cd65ad1` /
`a41d4efc4e838ca5b842b9492335a844900c38f9180d4335934345db0f448ba7`.
Footnote package/source SHA-256:
`daa074cc6bbac502a4f52f1cf17666ae262b8655d4daaa025f388af8fad064aa` /
`53b188431dd987fc768c957aad8ee20749105bf328ed89049cc579fa18f6531e`.
Logs: `/private/tmp/vmb-typaxis-table-regression.log`,
`/private/tmp/vmb-typaxis-table-public.log`,
`/private/tmp/vmb-typaxis-footnote-public.log`,
`/private/tmp/vmb-typaxis-footnote-regression.log`.

The public tests still combine real body contributions with a known test envelope;
these are not complete package export or build/PDF receipts. Full metadata/font/
style/page/outline/resource assembly, cross-reference and semantic container/figure
coverage, CLI/ArtifactSink integration, public common-flow closure, original
Harano and chapter/full-book/scale/both-host acceptance remain required. No public
profile was changed. All launched tests reached terminal state; no branch was
pushed in this work.

### 2026-09-07: VMB rich labels and deferred page references

Companion commit **`c087095e`** adds `package_reference.go`. Custom/title labels
retain rich inline content, math and language. Linked labels point to an emitted
internal anchor or resolved external URI; unlinked labels expand without a
fabricated emphasis/link wrapper. Copied title labels and locale-generated text
carry generated provenance. The sole `{page}` placeholder in the locale page
reference template becomes a wire page-reference node with an empty source span;
prefix/suffix text is retained without inserting guessed page digits. External
references already degraded by Prepare keep the resolved label and link state.

The exporter registers actual heading anchors and validates forward/internal
references after the full body and footnotes. Missing emitted targets, unresolved
page state, bad templates and unsupported payloads fail without partial output.
Recursive unlinked labels are bounded even when they create no wrapper nodes.
Number-only/label-and-number references still require genuine VMB/Typaxis counter
binding and fail explicitly; table/equation targets without emitted anchors also
remain unsupported. These are required remaining work, not optional exclusions.

Real source-bound VMB math in an unlinked label, rich linked text, forward links,
external fallback, generated provenance, exact node ceilings and fifteen negative
cases passed. Full rendertypaxis regression, including **five explicit public
check-package gates**, passed in **32.024 seconds**:

```sh
cd /Users/kazuyoshitoshiya/v/vmb-container/vmb-core
VMB_TYPAXIS_CLI=/private/tmp/typaxis-vmb-book-build/debug/typaxis \
VMB_TYPAXIS_FIXTURE_ROOT=/Users/kazuyoshitoshiya/t/typaxis/samples/machine-package/profiles/production-book-1/combined/job \
  go test ./internal/rendertypaxis/... -count=1 -v
```

The reference case has **1 image, 1 math occurrence, 71 projection bytes**, empty
diagnostics, package SHA-256
`26aa548aea5edfe2f19e0836ce99404808e3ca112e7774f39f7397b2b5114596`, source SHA-256
`f1fe322d13f5606f98eea14cd6390d908592569be7da8730139787181ebff02d`.
Binary SHA-256 remains
`2fe9e0cc5c233561ee3d29385e45b867c896d58ff6d753b383a1e5488f48677f`.
Prior four package/source hashes are unchanged. Logs:
`/private/tmp/vmb-reference-public.log`, `/private/tmp/vmb-reference-regression.log`.

This verifies a body contribution in the known test envelope, not final page
reference convergence, PDF links, formal complete package export or public build.
All previously recorded full-goal gates remain mandatory. All test processes
reached terminal state; no public profile was changed or branch pushed.

### 2026-09-07: VMB result, proof and exercise container traversal

Companion commit **`61548af3`** adds `package_semantic.go`. Result kinds retain
`semantic_kind=result`, proofs retain proof and exercises retain exercise. Their
real IDs become registered anchors. Existing author classes are retained, and the
type class is not duplicated. Statement/Blocks/Prompt use the common traversal.
Resolved heading prefixes are checked against the original logical number; rich
authored titles keep their source provenance. Generated heading wrappers, prefixes,
separators and locale proof-end text are marked generated. Proof targets become
internal links validated after traversal, including forward targets.

The implementation currently accepts the principal title/number/body fields and
proof Proves/QED. Assumptions, concept/prerequisite metadata, formal verification,
proof methods, exercise parts/choices/hints and answer/mode fields still fail the
populated-field guard. Their actual content and publication policy must be added;
this checkpoint does not remove them from the full objective. Heading/style policy
must still be integrated with the formal resolver.

Real source-bound inline/block math inside the three container kinds passed source
projection, rich-title, proof-link, QED, class/provenance and exact node-budget
checks. Eleven negative cases passed. All rendertypaxis regression tests, including
**six explicit public check-package cases**, passed in **41.954 seconds**:

```sh
cd /Users/kazuyoshitoshiya/v/vmb-container/vmb-core
VMB_TYPAXIS_CLI=/private/tmp/typaxis-vmb-book-build/debug/typaxis \
VMB_TYPAXIS_FIXTURE_ROOT=/Users/kazuyoshitoshiya/t/typaxis/samples/machine-package/profiles/production-book-1/combined/job \
  go test ./internal/rendertypaxis/... -count=1 -v
```

New case: **2 images, 2 math occurrences, 123 projection bytes**, empty diagnostics.
Package SHA-256: `18c0ce49fa53b29b8722459a1c57e7e15a2e2c56cdf62dd3f7fc1493eec2e1d3`.
Source SHA-256: `f842cab9de3fd27f68375144499b34c30c411d24dbd387d9ba915ac106f51070`.
Binary SHA-256 remains
`2fe9e0cc5c233561ee3d29385e45b867c896d58ff6d753b383a1e5488f48677f`.
Logs: `/private/tmp/vmb-semantic-public.log`, `/private/tmp/vmb-semantic-regression.log`.
This is admission of a body contribution in the known test envelope, not complete
package export, public build/PDF or full-book acceptance. All previously recorded
Harano/chapter/full-book/scale/both-host gates remain mandatory. Launched tests are
terminal; no public profile changed or branch was pushed.

### 2026-09-07: Actual prepared fractions-book inventory

Companion **`5fc26fac`** adds `tools/typaxis-book-audit`, a real canonical-build and
RenderBook preparation audit with explicit project/profile/locale, release mode
recorded in the report, and exclusive creation of output. It counts section content
without counting speech/alt again as visible inline occurrences. Root metadata and
other root collections are not folded into the section inventory. Nested content,
rich labels, table cells and map payload counting passed a focused unit test.

Current fractions v1 / profile.print / ja release preparation failed with **4,786
VMB-E0317** unit-review errors and **232 VMB-E0473** assessment approval errors; no
report or publication was created. These are observations about release mode, not
new approval requirements imposed on all PDF generation. Authored statuses were
not changed. An explicit **draft inventory** (`-release=false`) succeeded:

```sh
cd /Users/kazuyoshitoshiya/v/vmb-container/vmb-core
go test ./tools/typaxis-book-audit -count=1
go run ./tools/typaxis-book-audit \
  -project ../vmb-book-fractions-equivalence/v1/project.json \
  -profile profile.print -locale ja -release=false \
  -output /private/tmp/new-book-inventory.json
```

The report is tracked in the companion at
`vmb-core/tools/typaxis-book-audit/testdata/fractions-print-draft.inventory.json`.
Report SHA-256: `bb90768919c9f0fe70f24f34847627346ebbc938a3a7d56feea7c3a1ceae89d7`.
Canonical SHA-256: `ff25625dbf10f7abde790c6cf6d3279369958512ae8940c4e89940cb967df6e9`.
Source digest: `sha256:1b7ff2b6b607fd3d34cbef4b30500059dbcc71ff2a38622f01222414b2a0b436`.

Observed: **167 sections, 6,343 inline math, 278 block equations, 51 figures,
260 exercises, 285 proofs, 123 terms, 418 glossary entries**. All exercises carry
AnswerType/Difficulty/EstimatedMinutes/ExerciseMode; 252 proofs carry Methods.
These identify actual remaining exporter payloads rather than hypothetical feature
coverage. All 6,343 prepared inline math payloads lack explicit Speech/SemanticRef.
This does not yet prove absence in all underlying authored semantic records;
source assembly and semantic recovery must be investigated. Generic speech or raw
TeX cannot substitute for the required mathematical meaning.

This preparation does not match the saved **8,149-occurrence / 4,514-resource**
package and must be tracked separately. The smaller inventory is not a replacement
for the mandatory original full-book gate. No DocumentPackage or PDF was generated
by the audit. Logs: `/private/tmp/vmb-original-prepared-inventory.log`,
`/private/tmp/vmb-original-prepared-draft-inventory-v3.log`,
`/private/tmp/vmb-book-audit-tests.log` (unit test passed in 0.481 s). All launched
processes reached terminal state. Independent implementation work remains; the
goal is not blocked or complete, and no branch was pushed.

### 2026-09-07: Source-bound teaching metadata and speech-input trace

Companion **`16e96546`** retains concepts/prerequisites, proof methods and exercise
answer type/mode/difficulty/estimated minutes in `BookBody.Annotations`. Each
record binds original ID/type to its actual node ID, package pointer and source
origin, and owns its retained strings/arrays. Metadata is not inserted as invented
visible text. The final source-sidecar publisher must preserve these records;
that publisher is still outstanding.

Annotation limits bound retained UTF-8 payload (64 MiB by default, including source
identity) and records plus array elements (1,000,000 default). Checks precede
allocation, including empty-string arrays. Negative minutes, invalid UTF-8 and
exhausted limits fail without partial body/annotations. This is a component budget,
not closure of the still-pending global pipeline allocation budget. Actual
proof/result/exercise metadata passed source binding, input-mutation independence,
wire invariance, exact byte/item ceiling and ceiling-minus-one tests.

```sh
cd /Users/kazuyoshitoshiya/v/vmb-container/vmb-core
VMB_TYPAXIS_CLI=/private/tmp/typaxis-vmb-book-build/debug/typaxis \
VMB_TYPAXIS_FIXTURE_ROOT=/Users/kazuyoshitoshiya/t/typaxis/samples/machine-package/profiles/production-book-1/combined/job \
  go test ./internal/rendertypaxis/... -count=1 -v
```

All regression tests, including six explicit public admission cases, passed in
**42.399 s**. The semantic public case now includes metadata and retains the prior
package/source hashes from the container checkpoint. These public checks validate
wire admission, not publication of the metadata sidecar. Logs:
`/private/tmp/vmb-annotation-tests.log`, `/private/tmp/vmb-annotation-regression.log`.

Source tracing also found **7,739 raw mathInline declarations** across the current
project's listed topic files, with **zero speechUnit or semanticRef assignments**.
`render/inline.go` directly resolves the slot SpeechUnit and copies SemanticRef;
these missing direct assignments were not discarded during preparation. Block
math has separate authored speech units. Raw declarations, the prepared profile's
6,343 inline occurrences and the saved package's 8,149 formula occurrences remain
distinct quantities. No generic speech, TeX fallback or unverified association to
another semantic record was introduced.

Assumptions/formal verification, remaining exercise content, section/root metadata,
formal package/sidecar/CLI publication and all original PDF/full-book/Harano/scale/
both-host gates remain required. All launched tests reached terminal state; no
public profile changed or branch was pushed.

### 2026-09-07: Joined body/source encoding and private staging

Companion **`8bd8b92c`** adds `EncodeBookBody` and private staging. The encoder
validates every actual document node's pointer/parent/span and every text owner
against the projection. Actual tree math is collected and checked against the
completed math session; separate convenience lists cannot conceal changed visible
math. Annotations gain a process-local integrity fingerprint and are rechecked
against their source and semantic owner before encoding.

`vmb.typaxis-book-source-map/1` joins the exact body-document JSON hash, projection
URI/hash/length, node/text mappings, image declarations, math occurrences and
annotations. The body hash is not a full DocumentPackage hash. Encoding is
bounded, deterministic encoding/json output, not a JCS or full schema-admission
claim. Caller-defined serializers, cycles, invalid UTF-8 and excessive structures
are rejected before JSON allocation. Defaults bound document output to 64 MiB and
sidecar output to 256 MiB, with separate conservative preflight bounds; global
pipeline allocation closure is still required.

Encoded artifacts own document, sidecar and projection bytes. Private `Stage`
creates a new 0700 directory and exclusive 0600 files `body-document.json`,
`source/book-projection.txt` and `typaxis-source-map.json`, syncing and closing each.
Failure removes its owned temporary directory. Existing files are not overwritten.
Complete resources/package/config/math-export files, public CLI execution and
ArtifactSink publication are still outstanding.

```sh
cd /Users/kazuyoshitoshiya/v/vmb-container/vmb-core
VMB_TYPAXIS_CLI=/private/tmp/typaxis-vmb-book-build/debug/typaxis \
VMB_TYPAXIS_FIXTURE_ROOT=/Users/kazuyoshitoshiya/t/typaxis/samples/machine-package/profiles/production-book-1/combined/job \
  go test ./internal/rendertypaxis/... -count=1 -v
```

All regression tests, including six existing public body-admission cases, passed
in **51.926 s**. New checks cover joined content/hashes, deterministic bytes,
copy ownership, exact output ceilings and ceilings minus one, eleven altered or
executable inputs, table/footnote topology, cancellation, and exact staged files
after the original projection is changed. These public tests do not represent
admission/build of the new partial staging directory as a full package.
Logs: `/private/tmp/vmb-sidecar-tests.log`, `/private/tmp/vmb-sidecar-regression.log`.
All launched tests reached terminal state. No public profile was changed or branch
pushed; full-book, Harano, scale, both-host and public PDF/manifest gates remain.

### 2026-09-07: Bound math resource and sidecar staging

Companion **`5082f510`** adds `StageWithMath`. The encoded body records the exact
completed math-session fingerprint; staging accepts only the matching unchanged
session. SVGs are written from that revalidated session without retaining another
whole image-set copy, and each saved file is re-read and checked against its
content hash. The session is revalidated at completion. Callers must not mutate it
concurrently. Paths are the validated content-addressed resource URIs.

`typaxis-math-export.json` is emitted record by record with the established DTO
bytes, excluding SVG payloads and process-local authority. Its explicit byte limit
is checked before writes. Failed writes, limit exhaustion or cancellation clean
the owned staging directory. A decoded DTO or another body's completed session
cannot be substituted. Font/ordinary-image resources and full package/config
assembly are still required before this becomes a valid job.

Tests exercised two shared SVG resources across three actual math occurrences,
exact saved bytes and JSON, exact sidecar ceiling and ceiling minus one, cleanup,
changed SVG/URI/occurrence data, foreign session, decoded DTO and cancellation.
A new public check reads the staged document/source/SVG files back into the known
test envelope. All regression tests, including **seven public admission cases**,
passed in **54.418 s**:

```sh
cd /Users/kazuyoshitoshiya/v/vmb-container/vmb-core
VMB_TYPAXIS_CLI=/private/tmp/typaxis-vmb-book-build/debug/typaxis \
VMB_TYPAXIS_FIXTURE_ROOT=/Users/kazuyoshitoshiya/t/typaxis/samples/machine-package/profiles/production-book-1/combined/job \
  go test ./internal/rendertypaxis/... -count=1 -v
```

New case: **2 images, 3 math occurrences, 131 projection bytes**, empty diagnostics.
Package SHA-256: `c13feed60e551e7a2dc63c83605837d9b746c8d72ddd24d49105faaccd76dabc`.
Source SHA-256: `99aa44bafc82398327482f730868976b7ad84915a22f8f3ec729e3ef81feb8c7`.
Binary SHA-256 remains
`2fe9e0cc5c233561ee3d29385e45b867c896d58ff6d753b383a1e5488f48677f`.
Logs: `/private/tmp/vmb-math-staging-tests.log`,
`/private/tmp/vmb-math-staging-regression.log`.
This is private input staging plus a test-envelope check, not formal complete
package assembly, public build/PDF or ArtifactSink publication. Full original-book,
Harano, distinct/alias scale, both-host and public manifest gates remain mandatory.
All launched tests reached terminal state; no public profile changed or branch
was pushed.

### 2026-09-07: Prepared-book publication metadata

Companion **`8591c9e2`** adds `EncodeBookMetadata`: title/subtitle, author names,
description, identifiers and keywords are derived from RenderBook. An independent
`vmb.typaxis-book-metadata/1` record retains publisher, contributor roles, rights,
all identifiers and rich source metadata. Dates are explicit valid UTC seconds or
null; publication dates are not turned into invented midnight timestamps.
Modification-before-creation, unresolved dynamic references, missing math speech,
invalid controls and ambiguous/unknown inline payloads fail without partial output.

The initial public check found the existing strict keyword-order rule. The encoder
now emits unique keywords in UTF-8 ascending order while preserving original order
and duplicates in the source record. The other package regression cases passed;
corrected metadata unit/public tests passed in **1.746 s**. Tests cover retained
fields, explicit presentation, byte ceilings, owned bytes and invalid inputs.

```sh
cd /Users/kazuyoshitoshiya/v/vmb-container/vmb-core
VMB_TYPAXIS_CLI=/private/tmp/typaxis-vmb-book-build/debug/typaxis \
VMB_TYPAXIS_FIXTURE_ROOT=/Users/kazuyoshitoshiya/t/typaxis/samples/machine-package/profiles/production-book-1/combined/job \
  go test ./internal/rendertypaxis -run 'TestEncodeBookMetadata|TestBookMetadataPublicCheckPackage' -count=1 -v
```

The public test replaces fixture metadata with the actual prepared example's
metadata: **2 images, 2 formula occurrences, 123 projection bytes**, empty diagnostics.
Package SHA-256: `b0252a26f069f6574bd1a3156db6ab4c537f8af217369e4aa9756d1fa0796736`.
Source SHA-256 remains `f842cab9de3fd27f68375144499b34c30c411d24dbd387d9ba915ac106f51070`.
Logs: `/private/tmp/vmb-metadata-regression.log` (includes initial keyword failure),
`/private/tmp/vmb-metadata-final.log` (corrected success). This is metadata encoding
and test-envelope admission, not the final assembler, source-record publication or
PDF/full-book/Harano/scale/both-host acceptance. All launched tests are terminal;
no public profile changed or branch was pushed.


### 2026-09-07: Outline from actual body hierarchy and labels

Companion **`9dc8d4c2`** adds `EncodeBookOutline`. It derives headings and explicitly
selected semantic containers from the validated body tree, orders entries by their
source node IDs and assigns dense outline IDs and exact parents. Labels retain
source text and verified math ActualText. Missing hierarchy levels, destinations
and unresolved dynamic labels fail without partial output. Entry and byte ceilings
are enforced; returned bytes are owned copies.

The complete rendertypaxis regression suite passed in **58.874 s**, including nine
public admission cases. The new outline case has **2 images, 3 math occurrences,
146 projection bytes** and empty diagnostics. Package SHA-256:
`8ca1446049433289378a90429f9724bf11f172ed77ce622ecb65165b1f5654a3`.
Source SHA-256: `965ef4bca1e805db4ffa22c9909ba35f25644d0d2e23153d74d21664e8e4f29e`.
Logs: `/private/tmp/vmb-outline-tests.log`, `/private/tmp/vmb-outline-regression.log`.
Command and CLI hash are recorded in companion design §15.34. This remains a
test-envelope check of actual metadata/outline contributions, not the complete
assembler, resolved PDF navigation or full-book/Harano/scale/both-host acceptance.
All launched tests reached terminal state; no public profile changed or branch pushed.


### 2026-09-07: Resolved paper and explicit page regions

Companion **`3357fc78`** adds `EncodeBookPages`. Resolved A4/A5/ISO B5/Letter/6x9in
and custom paper dimensions become one horizontal LTR page master. Margins and
footnote height are explicit policy because RenderBook's resolved page has no
margin fields. Millimetres use exact 360/127 pt conversion and nearest/tie-to-even
rounding. A4 landscape is **55,174,088 × 39,011,981 raw units**; 54pt margins are
3,538,944 raw each. Invalid or exhausted regions fail without changing content size.

The new public check combines actual footnote body, metadata, outline and the
unmodified generated A4 landscape page master. **1 image, 1 formula occurrence,
58 projection bytes**, empty diagnostics. Package SHA-256:
`c4071cf626838dd4d67fb2a1e92a5a1e98ea5b389b5c6ecf6360aba411b77d05`.
Source SHA-256: `53b188431dd987fc768c957aad8ee20749105bf328ed89049cc579fa18f6531e`.
An initial test constant typo was corrected using independent Python Fraction
calculation; its log remains at `/private/tmp/vmb-pages-tests.log`. All package
regression tests, including **10 public admission cases**, then passed in
**56.601 s** (`/private/tmp/vmb-pages-regression.log`; command in companion §15.35).

This is page-master generation and admission. Font/style integration, actual-frame
math fit checks, named pages and generated page regions, complete assembler, public
PDF and original-book/Harano/scale/both-host gates remain required. All launched
tests are terminal; no public profile changed or branch was pushed.


### 2026-09-07: Explicit font selection and exact snapshot staging

Companion **`f90e1b19`** adds `ReadBookFont` and `SelectedBookFont.Stage`. Absolute
path, family, published profile and a positive byte ceiling are explicit. A
bounded regular file is read twice through the same descriptor, compared byte for
byte, and checked against the initial/current file identity, size and mtime.
Optional expected SHA-256 is checked before classification. Nonblocking open
avoids waiting on a replaced FIFO. This observation is not an OS snapshot against
a malicious concurrent writer; saved artifacts use only the owned snapshot.

TT/TTC/name-keyed CFF container and table-directory classification is separate from
authoritative Typaxis font admission. TTC requires an explicit index even for
zero. CFF ROS detection respects operand boundaries and rejects the original
Harano CID font under current book-1. No alternate font/profile is substituted.
Full tables/checksums, embedding rights, glyph coverage and rendering remain
Typaxis admission/build responsibilities. Content-addressed font files are saved
exclusively and rehashed; the assembler owns failed-job cleanup.

All regression tests passed in **56.568 s**, including 11 initial public admission
cases. The font public test was then expanded to TT, TTC and name-keyed CFF; all
three passed in **1.440 s**. Each has 2 images, 2 math occurrences, 88 projection
bytes and empty diagnostics. Package SHA-256:

- TT: `eeeab65567cc00a71c8816e52169d19138463933b2fac684511950c54e93df59`
- TTC: `87c10b8711a34783a39db2922baaa3e18d8e2c19c051141e37bfe4918875f197`
- CFF: `4f7fa794f0fa1d70783fad8ea81d6d37d2d7ec77ce90f8940156fdbbcd4d7cce`

Logs: `/private/tmp/vmb-font-tests.log`, `/private/tmp/vmb-font-regression.log`,
`/private/tmp/vmb-font-public-final.log`; commands in companion §15.36. Tests also
cover limits, hash/path/face/profile errors, malformed directories, FIFO, cancel,
CFF operand boundaries and snapshot preservation after host-file replacement.
All runs are terminal. Complete style/config/package assembly, public PDF and
original-book/Harano/scale/both-host gates remain. No profile changed or push ran.


### 2026-09-07: Package assembly without a fixture envelope

Companion **`61005bdd`** adds `StageBookPackage`. The same prepared book owns body,
metadata, outline, pages, text/source records, real math resources and explicitly
selected font. All package members are generated; none come from a fixture
package. The initial explicit uniform font/size/line-height policy applies to
paragraphs, headings and list markers, with paragraph rules covering text inside
semantic containers. Unsupported root glossary/bibliography/verification pages
are rejected rather than omitted. Footnotes require an explicit page region.

A `vmb.typaxis-book-export/1` record binds book identity, original resolved profile,
explicit layout/font settings, package hash and metadata-source hash. Package
bytes are bounded during record emission. Tests cover exact package ceiling and
ceiling minus one, determinism, joined resources, root rejection and cleanup after
math-sidecar staging has begun. All regression tests passed in **61.072 s**,
including **14 public admission cases**. A later cleanup extension passed in
**2.211 s**. Logs: `/private/tmp/vmb-package-regression.log`,
`/private/tmp/vmb-package-cleanup-final.log`, `/private/tmp/vmb-package-public-v5.log`.

Saved generated job: `/private/tmp/vmb-assembled-package-smoke/typaxis-body-335423113`.
Package SHA-256: `abc08e432448f3464934d60ea18cbd06c9425657ac59d73f330ce337070aaeec`.
Source SHA-256: `fe9f74e6f7aaa1bdb78945e828cd4866ad6801bb4bcc3780b25de65f5669a6e3`.
It contains 2 images, 2 math occurrences and 88 projection bytes. Public check
passed with empty diagnostics. Its explicit test config is not yet the complete
backend policy config. This is an authored body-only diagnostic fixture, not the
original whole-book acceptance input.

One public build of the **same input bytes** failed with exit **4**,
`I9190: production tagged-PDF native math mismatch`; no PDF was emitted. Package,
config, source and every declared resource hash matched before/after. Full command,
hashes and result: `/private/tmp/vmb-assembled-build.log`; diagnostics remain in
the saved job. The public writer still has a path that requires a native-math
font for standard text. This observation does not isolate the precise failing
return site. Connecting genuine common body/math selection to public terminal,
paint and manifest remains required; dummy native math is not a repair.

Full RenderBook coverage, complete config/process/publication, original-book,
Harano, scale, both-host and independent PDF gates remain. All processes are
terminal; no public profile was changed or branch pushed.


### 2026-09-07: Common body lifetime owner through stable selection and assembly

`with_production_common_body_pdf` joins existing genuine body flow/bindings/math
registry, selected-line reshape feedback, pagination, block math terminal
completion, display/font use, marked structure, objects and PDF assembly under
one borrowing lifetime. Only a successfully verified final assembly reaches the
callback. It reports actual pass count, candidate work, selected/flow hashes and
completed block count. It neither requests a synthetic native-math font nor
issues a public VerifiedPdfBytesReceipt. Public terminal/paint/manifest closure,
page/generated convergence, final bidi and full cumulative allocation remain.

New tests cover native-math-free text plus inline/block vectors, numbered blocks,
at least two real reshape passes, repeat determinism and no consumer invocation
when candidate budget is exhausted. Saved-job inspection is an explicitly ignored
test requiring an absolute input job and exclusive diagnostic PDF output; it
checks the original source files and admits the declared resource bytes. The
normal production regression finished with **94 passed, 0 failed, 1 ignored** in
**262.51 s**, including both 5,000-resource page-content tests.

```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_ --locked -- --nocapture
```

Logs: `/private/tmp/typaxis-common-driver-final.log` (focused tests),
`/private/tmp/typaxis-common-driver-regression.log` (regression). Initial compilation
failed on include-file inner doc comments; corrected comments and include location
compiled successfully. The long-running tests were observed live, not restarted.

The actual VMB assembler job with the admission-only fixture font failed real
shaping at **node 2, text 0, bytes 0..1** (the heading prefix `1` had no glyph).
Log: `/private/tmp/typaxis-common-vmb-job.log`. That input remains unchanged.
Companion **`3e1d9b01`** adds an explicit optional test font path. A separately
generated job selects local Arial Unicode (not redistributed):

- Job: `/private/tmp/vmb-assembled-package-smoke/typaxis-body-2451059118`.
- Package SHA-256: `fe97596c3c205df5f8ca1c4242b6013f267a8f7bc73a01a64490937c70975a02`.
- Font SHA-256: `876af2cd4854644e7f3e7feb2f688997fdb3343c6df6693611209c9dfb47ccec`.
- Source SHA-256: `fe9f74e6f7aaa1bdb78945e828cd4866ad6801bb4bcc3780b25de65f5669a6e3`.

This generated package passed public check, then produced **one diagnostic page**
through the common owner with **2 reshape passes, 84 candidate steps, 1 block
terminal**, no native math. PDF: `/private/tmp/vmb-common-real-font-diagnostic.pdf`,
SHA-256 `d534fdbacb4eecea2ae70f0a19173138217eb24e9f49a07f2e4a58eac7bd267a`.
The ignored test was invoked with `TYPAXIS_COMMON_BODY_JOB` set to that job,
`TYPAXIS_COMMON_BODY_PDF` set to the diagnostic output, and cargo test filter
`production_common_driver_saved_vmb_job -- --ignored --nocapture`.
Log: `/private/tmp/typaxis-common-real-font-job.log`; successful inspection 1.27 s.

Poppler **26.08.0** and MuPDF **1.28.2** extracted the source-order heading, body,
ordered-list numbering and formula ActualText, equal after whitespace normalization.
Both renderings visibly contain the Japanese body and two formula outlines.
There are two Formula structure nodes. This input has **no equation number**;
numbered-block coverage belongs to the separate unit fixture. pdffonts reports
embedded CID TrueType with Unicode and a Type 3 resource. This is not a full
independent tagged-PDF audit or an exact whitespace/baseline comparison.
Report: `/private/tmp/vmb-common-independent-report.json`; script:
`/private/tmp/verify-vmb-common-diagnostic.py`. Renderings:
`/private/tmp/vmb-common-page.png`, `/private/tmp/vmb-common-mupdf-page.png`.

The **same real-font package/config/resources** still fail public build with
**exit 4 / I9190**, without a public PDF. Input hashes before/after match.
`/private/tmp/vmb-real-font-public-build.log` records the command and hashes; public
binary SHA-256 is `ff3b1022fa79d949b0a66f533aa0a0d9976bc41b38193dc037b6765d539bb65e`.
The common diagnostic success must not be relabeled as public build success.
Original whole-book, Harano, real distinct/alias scale, both-host and public PDF/
manifest gates remain mandatory. All launched builds/tests are terminal. No public
profile was changed, no font/PDF binary was committed, and no branch was pushed.


### 2026-09-07: Static page stability and cumulative pass records

`paginate_stable_production_body` now compares at least two actual page searches
from the same immutable selected lines and blocks. It compares full selection
fingerprints, including placements and break decisions, and honors
`max_layout_passes`. Each pass's ordinary input/output record charges plus one
observation are conservatively accumulated against the same `max_fragments`.
Earlier work is not refunded. The existing single-pass API retains its behavior.

`ProductionBodyPageStability` is a non-constructible external proof bound to the
exact borrowed line/block preparations. It rejects another preparation and a
single-pass selection with matching placement bytes but missing cumulative work.
It verifies the original placement fingerprint after block math terminals are
attached. The common PDF owner retains it, verifies after terminal completion and
after assembly, and passes the same typed proof to its callback. It remains a
static-body proof, not generated-reference or complete global convergence.
Dynamic reference/footnote-reference sites are rejected at their source owners.

Tests demonstrate 40 records for the existing single pass, 82 for two passes plus
observations, success at 82 and rejection at 81/41, pass-limit rejection at 1 and
success at 2, foreign preparation rejection and verification after terminals.
Final focused page tests: **3 passed** in **0.58 s**; common-owner tests: **2 passed,
1 explicitly ignored** in **0.65 s**. Logs:
`/private/tmp/typaxis-page-feedback-final.log`,
`/private/tmp/typaxis-page-feedback-common-final.log`.

The saved real-font VMB job from the previous entry was explicitly rerun through
the final callback API: **2 line reshapes, 2 page passes, 266 cumulative page
records, 84 line candidate steps, 1 block terminal, 1 page**. The new diagnostic
PDF `/private/tmp/vmb-common-page-feedback-diagnostic.pdf` is byte-for-byte equal
to the prior independently extracted/rendered PDF, SHA-256
`d534fdbacb4eecea2ae70f0a19173138217eb24e9f49a07f2e4a58eac7bd267a`.
Log `/private/tmp/typaxis-page-feedback-real-job.log`; explicit job test **1 passed**
in **1.44 s**. This exact-byte comparison preserves the earlier limited extraction
and rendering evidence; it does not expand that evidence to whole-book acceptance.

Page/generated text convergence, named pages, complete subflows, final bidi, all
pipeline allocation and public terminal/paint/writer/manifest closure remain.
The existing public I9190 failure is not repaired or hidden by this static stage.

The production regression also completed: **97 passed, 0 failed, 1 ignored** in
**271.19 s**, including both 5,000-resource cases. Command:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_ --locked -- --nocapture
```
Log: `/private/tmp/typaxis-page-feedback-regression.log`. The subsequent callback
proof exposure/source-owner refinement was covered by the focused and explicit
real-job checks above. All launched processes are terminal. No public profile
changed or branch was pushed.


### 2026-09-07: Borrowed reference identity in the common body flow

Added `ProductionInlineReference` and `ProductionReferenceFormat` to the common
syntax flow. Ordinary references retain the actual target spelling, requested
text/page/number format and target owner from the same validated navigation
registry (binary search over its sorted anchor list). Footnote references retain
the actual borrowed footnote ID. Non-reference sites and synthetic container ends
carry no reference payload. No generated label, counter value or page is guessed.

The flow's private fields and reconstruction-based verification reject changed
targets, target owners, formats, reference kinds and footnote IDs. The existing
fingerprint already binds the entire package SHA and navigation-derived state;
this deterministic source projection does not change the algorithm identifier.
The existing unresolved-reference refusal remains in downstream layout stages.
This change does not implement generated-text shaping or page/reference convergence.

Validation:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-syntax --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked -- --nocapture
```
Syntax: **67 passed** in **0.97 s**, plus **6 doc-tests passed** in **1.16 s**.
Common owner: **2 passed, 1 explicitly ignored** in **0.75 s**. Logs:
`/private/tmp/typaxis-reference-carrier-syntax.log` and
`/private/tmp/typaxis-reference-carrier-common.log`. The explicit saved-VMB job
was not rerun for this source-identity-only addition. All launched tests completed.
No public profile, PDF writer or VMB exporter behavior changed. Original full-book,
Harano, scale, independent PDF and both-host acceptance gates remain outstanding.


### 2026-09-07: Definition-order footnote labels in common paragraph shaping

The common flow now derives canonical decimal footnote labels from validated
footnote-definition order. Every reference and definition gets its own
`FootnoteMarker` generated key/buffer; repeated references do not share a source
owner. The body shaper uses actual label bytes in paragraph itemization and shapes
with the paragraph's admitted font, language and line context. Marker-only
paragraphs obtain genuine font/glyph output. Final-context reshaping preserves
generated subspans; the fingerprint encodes parsed and generated sources distinctly.
Footnote owners remain pending because their definition regions/page placement
have not been connected. No reference is silently treated as empty text.

A text-budget boundary test exposed the old common flow's use of text-buffer
bytes alone as retained input. The syntax package now preserves its actual parser
charge for buffers/native speech/vector alternatives and explicit vector language.
The common generated-text budget adds metadata, outline labels and navigation
language charge, excluding vector language already charged by the parser. List and
footnote generated bytes share the remaining text budget, checked before label
allocation. Exact-fit input succeeds; one byte less rejects the last footnote
marker at definition owner 92. This closes this text-budget boundary, not the
outstanding pipeline-wide allocation budget.

Internal identities become `typaxis.production-text-flow/6` and
`typaxis.production-authored-text-shape/4`. Public identifiers, old writer and
VMB exporter behavior remain unchanged.

Validation commands:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-syntax --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_authored_text --locked -- --nocapture
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_ --locked -- --nocapture
```
Syntax: **69 passed** in **0.83 s**, plus **6 doc-tests passed** in **2.02 s**.
Focused body shaping: **6 passed** in **0.36 s**. Tests include reverse reference
versus definition order, distinct generated owners, changed-label rejection,
combined text boundary, a two-digit marker-only paragraph, real nonzero font GIDs,
and generated subspans after explicit line-context splitting. These explicit
contexts exercise reshape mechanics; they are not a converged line/page proof.
Logs: `/private/tmp/typaxis-footnote-generated-syntax-all.log` and
`/private/tmp/typaxis-footnote-generated-shape-final.log`.

The added multi-footnote fixture initially failed closed-carrier, node-order and
outline/profile checks; its source node IDs, outline references and all footnote
references were corrected. No production input was relaxed to pass those checks.
The shared-text boundary first exposed parser speech and navigation charges, which
are now carried into the common flow instead of raising the test's limit.

Page-reference/counter resolution, footnote definition placement, full-flow
convergence, final bidi, public terminal/paint/writer/manifest closure and original
full-book/Harano/scale/both-host acceptance remain incomplete.

The full production regression completed: **98 passed, 0 failed, 1 explicitly
ignored** in **244.38 s**, including both 5,000-resource selected PDF cases.
Log: `/private/tmp/typaxis-footnote-generated-regression.log`. The ignored saved-VMB
job was not requested by this run. All launched test processes are terminal;
subsequent edits only formatted the new test and clarified comments/documentation.
No branch was pushed and no full-book/public PDF success is claimed.


### 2026-09-07: Generated footnote glyphs through selected inline lines

Common inline preparation now consumes canonical footnote glyph clusters along
with parsed body clusters. Selected clusters retain `ShapeSourceSpan`, preserving
the generated key, buffer ID and byte range instead of forging a parsed TextSpan.
The shared range check rejects crossed namespaces, foreign owners and out-of-range
subspans. Existing parsed spans with nonzero starts use relative offsets correctly.

The line-context projection counts the actual generated label bytes. A real
footnote fixture exercises 1-line and 2-line selection of `A 1B`, the actual selected
font/glyph/baseline, and a true select/reshape/rebreak stable callback. Page selection
still refuses unsupported footnote placement: the ordinary paginator rejects the
definition at node 6 and the static stability owner rejects its reference at node 4.
No footnote definition is flattened into ordinary body placement.

The body display projection can retain generated provenance and uses the same
parsed-buffer-count offset convention as generated list labels. Its checked parsed
buffer count is cached on first use, avoiding per-cluster package revalidation.
This branch does not issue footnote structure or PDF authority; complete footnote
page/structure ownership remains outstanding. Internal identities become inline
preparation `/5`, selected inline layout `/4`, and body display `/6`.

Commands:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-layout \
  generated_ranges --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_line_projection --locked -- --nocapture
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_ --locked -- --nocapture
```
Range negative test: **1 passed**. Focused projection tests: **7 passed** in
**0.24 s**. Logs `/private/tmp/typaxis-footnote-ranges-tests.log` and
`/private/tmp/typaxis-footnote-lines-tests.log`.

The initial preparation edit exposed a source-byte versus logical-unit offset
shadowing error; the source-end variable was separated and the existing mixed
text/vector and explicit-break cases passed after correction. The convergence
fixture uses a valid body frame; separately supplied direct line widths exercise
its one- and two-line cases. Neither fixture proves footnote-region pagination.

Production regression completed: **99 passed, 0 failed, 1 explicitly ignored**
in **263.44 s**, including both selected 5,000-resource PDF cases. Log:
`/private/tmp/typaxis-footnote-lines-regression.log`. All launched tests are
terminal. The explicit saved-VMB job was not run in this regression. No branch
was pushed. Footnote page/structure closure, dynamic page/counter convergence,
public writer/manifest, original full-book, Harano and both-host acceptance remain.


### 2026-09-07: Footnote definition ranges and selected reference positions

Added `ProductionFootnoteLines`, built from the actual selected lines and the same
validated wire/flow. Definitions retain borrowed source IDs, owners, canonical
numbers and their exact event/paragraph ranges. References join to definition
indices, retain body-versus-definition source scope, and record the first/last
selected cluster positions. Coverage checks require the complete canonical
generated marker, with matching key/buffer/range and bytes. Repeated references
remain distinct source owners and placements.

The registry has private fields and borrows the exact selected-line owner.
Verification rejects a separate selection even when it has the same contents.
Definition and reference/coverage records are charged on top of selected output
before allocation; a one-definition/one-reference input requires 3 additional
records. Exact fit succeeds and one less rejects. Stable line callbacks own this
registry, and the common PDF owner verifies it before attempting pagination.
This does not assign footnotes to pages, prove region fit, or close publication.
The existing page rejection remains; the future page owner must retain the
registry's cumulative charge when integrating actual definition placement.

Focused projection/source-binding checks: **7 passed** in **0.37 s**, including
record boundaries, foreign selection rejection, actual line positions, source
ranges and callback ownership. Log `/private/tmp/typaxis-footnote-registry-tests.log`.
Repeated-reference check initially passed in **0.11 s**; the final focused run also
adds a ten-definition input to exercise all clusters of the two-digit marker.

Commands:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_line_projection --locked -- --nocapture
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote_line_registry --locked -- --nocapture
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_ --locked -- --nocapture
```
Original full-book, Harano, complete dynamic reference/footnote convergence, public
writer/manifest and both-host acceptance gates remain outstanding.

Production regression: **100 passed, 0 failed, 1 explicitly ignored** in
**294.14 s**, including both selected 5,000-resource PDF cases. Log:
`/private/tmp/typaxis-footnote-registry-regression.log`. The additional multi-digit
test was added after that regression binary was built; the final focused registry
run covers both repeated and multi-digit references: **2 passed** in **0.16 s**,
log `/private/tmp/typaxis-footnote-registry-final.log`. No production implementation
changed after the regression began. All launched tests are terminal. No branch
was pushed and no new full-book/public PDF result is claimed.

### Declared footnote geometry in common inline frames

The common frame owner now binds definitions to the actual single default master
and its page-contained footnote rectangle. An unrelated caller body, missing
footnote region or unselected master policy fails explicitly. A definition starts
from the declared width and body-relative x, then applies ordinary paragraph and
nested frame indents. Leaving the definition restores the previous frame.
The retained optional maximum rectangle is charged and hashed under internal
`typaxis.production-body-frames/2`; it is not a selected page/height receipt.

Focused verification covers a narrower definition wrapping while body remains on
one line, offsets to both sides of the body origin, changed maximum height,
independent successive definition frames, actual reshape convergence, and missing
region / mismatched caller body rejection. The existing selected-line projection
suite passed **7 tests** in **0.31 s**; initial region test passed **1 test** in
**0.43 s**, before adding the successive-definition case.

Command for final regression:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_ --locked
```
Log: `/private/tmp/typaxis-footnote-frame-regression.log`.
Page assignment, collision/reservation/continuation, definition-marker paint,
public writer and all original full-book/Harano/both-host gates remain open.

Final regression terminated in **274.61 s**: **101 passed, 1 failed, 1 ignored**.
Both 5,000-resource cases passed. The sole failure was the newly added fixture's
unreferenced second definition, rejected at semantic admission before layout.
Adding its actual source reference and renumbering source owners fixes the input;
no production implementation changed after the broad run began. The corrected
focused test passed **1 test** in **0.48 s**, including two successive definitions,
log `/private/tmp/typaxis-footnote-frame-final.log`. The broad run is recorded as
failed rather than rewritten as a clean run. All test processes are terminal.
No new public/full-book PDF or branch push is claimed.

### Shared measured body and footnote-definition item collection

`prepare_production_body_flow` borrows actual selected lines, vector-block
preparation and the existing footnote-line registry. Shared admission/epoch/
limits checks reject unrelated inputs, and definition collection requires actual
footnote frames. It reuses the ordinary page collector and list metric expansion,
retaining each definition's measured items separately from body items. The
registry's exact owner/event ranges delimit the slices. Source indices, spacing,
keep flags, forced breaks, real block metrics and tall marker extents survive.
There is no page/height/continuation or paint receipt yet.

Focused tests use a real VMB vector block plus a nested list in one definition,
a second definition, actual source references and large list-marker glyphs.
They verify source joins, no body splice, definition boundary isolation, marker
height expansion, exact cumulative record budget / one-record shortage, foreign
selected lines / block preparations / registries, missing footnote frames, and
an explicit break retained through the real stable-line callback. Initial test
inputs needed corrected reference source spans and sufficient declared width
for the intentionally large nested markers; admission/frame rejection remained
intact. Final focused run: **2 passed, 0 failed**, **2.54 s**.
Log: `/private/tmp/typaxis-footnote-items-final2.log`.

Commands:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote_flow --locked -- --nocapture
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_ --locked
```
Regression log: `/private/tmp/typaxis-footnote-items-regression.log`.
Footnote page assignment/reservation/continuations, public writer and full-book,
Harano, scale and both-host acceptance remain required and incomplete.

Final production regression: **104 passed, 0 failed, 1 explicitly ignored** in
**701.63 s**, including both selected 5,000-resource PDF cases. The same process
was retained through the longer run; CPU/progress observations and a short stack
sample showed active PDF content/structure work, not a terminal timeout. The
sample is `/private/tmp/typaxis-footnote-items-sample.txt`. All launched tests
and sampling processes are terminal. Only API documentation comments changed
after the regression binary was started. No branch push or new full-book/public
PDF acceptance result is claimed.

### Footnote content-fragment boundary search and continuation cursors

The shared page-boundary kernel now exposes a page-independent boundary choice.
Ordinary body pagination wraps the same choice with its actual page index;
footnote content selection does not manufacture a page assignment.
`ProductionFootnoteBreakSearch` borrows the measured flow and reuses paragraph
lengths/heading context once. Opaque source-bound cursors start each definition
and advance from actual selected ranges, including consumed forced breaks.
Repeated candidate evaluation shares cumulative candidate records and visited
item work. No-fit at a smaller capacity differs from hard Oversize at the actual
declared maximum. Negative / above-maximum capacity is rejected.

Four focused tests verify continuation without content loss/duplication, foreign
flow/cursor rejection, exact records and visited-work limits, non-refunded retries,
exact candidate-lookback rejection, leading/consecutive/trailing forced breaks,
keep chains, inter-paragraph spacing, keep across a forced break, and an actual
VMB block exceeding the maximum region. **4 passed, 0 failed, 0.60 s**;
log `/private/tmp/typaxis-footnote-selection-final.log`.
The shared boundary-kernel regression passed **7 tests**, covering measured
widow/orphan choices, headings/keeps, spaces/marker extents, tie ordering,
candidate limits, continuation and forced blank pages;
log `/private/tmp/typaxis-footnote-selection-pagination.log`.

Commands:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote_breaks --locked -- --nocapture
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-pagination \
  production_body --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_ --locked
```
Production regression log: `/private/tmp/typaxis-footnote-selection-regression.log`.
This selects measured content only. Definition-marker glyph metrics and spacing
between definitions must join final reservation; reference-page assignment,
body/footnote candidate coupling, continuations in page state, structure/public
PDF and original full-book/Harano/both-host acceptance remain open.

Final production regression: **108 passed, 0 failed, 1 explicitly ignored** in
**676.00 s**, including both selected 5,000-resource PDF cases. The same process
was retained and observed active throughout the longer run. After its binary
started, only a definition-index accessor, formatting and documentation comments
were added; the final pagination crate passed `cargo check -p typaxis-pagination
--locked` with the same manifest/target directory (log
`/private/tmp/typaxis-footnote-selection-final-check.log`). Boundary selection,
work/record accounting and tests did not change after the broad run began.
All launched commands are terminal. No branch push or new public/full-book PDF
acceptance result is claimed.

### Actual admitted-font shapes for footnote definition numbers

The flow now retains definition-order marker sources and binds each marker to
its first actual paragraph/heading's computed style and language. A definition
without paragraphs uses the declared classless paragraph base style, resolved
once; it never takes the first admitted font or a synthetic font size. The source
keeps actual definition ID/owner and paragraph-style index. Reconstruction rejects
style-index or language tampering.

Footnote and list labels share the actual admitted-font shaping kernel, including
coverage, metrics, backend/cluster validation, missing-glyph rejection and retained
record charging. Definition markers retain source pointers, generated provenance,
real glyphs/advances and font metrics through the selected-line owner. Marker
fingerprints now encode generation kind and owner-local ordinal. Definition marker
records are also carried once into selected-line accounting. Internal algorithms:
text flow **/7**, authored text shape **/5**, inline line layout **/5**.

Focused marker tests: **3 passed, 0.20 s**, including a definition-only font-size
change, vector-only definition base style, deterministic shapes, exact shape
record budget / one-record shortage, and digit coverage failure at the actual
definition owner using the CFF fixture font. The multi-digit registry test now
also checks the actual two-cluster definition marker "10". Final combined
footnote tests: **12 passed, 0 failed, 0.85 s**;
`/private/tmp/typaxis-footnote-marker-final.log`.
Full syntax tests: **69 passed** in **0.96 s**, plus **6 compile-fail doc tests**
in **2.54 s**; `/private/tmp/typaxis-footnote-marker-syntax-all.log`.

Commands:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked -- --nocapture
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-syntax --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_ --locked
```
Production regression log: `/private/tmp/typaxis-footnote-marker-regression.log`.
The marker column, baseline and vertical extents still need to join footnote
frames/items before final reservation. Reference-page coupling, public paint/PDF,
original full-book/Harano/scale and both-host gates remain incomplete.

Final production regression: **111 passed, 0 failed, 1 explicitly ignored** in
**266.28 s**, including both selected 5,000-resource PDF cases.
No production implementation changed after the regression began. All launched
commands are terminal. No branch push or new public/full-book PDF acceptance
result is claimed.

### Footnote number columns reserve actual content width

Declared footnote frames now reserve a shared number column using the maximum
actual shaped definition-marker advance, plus a gap equal to the maximum marker
font size (one em). Each definition column binds its source owner; paragraphs,
vector blocks and nested lists inherit the reduced content frame. Each definition
restores the preceding frame. A nonpositive remaining width reports
`FootnoteFrameExhausted` at the definition owner. Column records are charged once
and their geometry is fingerprinted; internal body frames advance to **/3**.

Focused verification: **13 passed, 0 failed, 1.53 s**;
`/private/tmp/typaxis-footnote-column-verified.log`. Coverage includes actual
multi-digit advances, mixed definition font sizes, shared alignment, both region
x offsets, descendant indents, definition restoration, exact exhaustion and
one-unit shortage. Narrow fragmentation fixtures now explicitly include room
for the number column; their keep, continuation and budget assertions still pass.

Commands:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked -- --nocapture
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_ --locked
```
Production regression log: `/private/tmp/typaxis-footnote-column-regression.log`.

Marker baseline/vertical extents and paint placement remain to be connected to
the measured items. Reference-page coupling, public PDF, original full-book,
Harano, scale and both-host gates remain incomplete. No public/full-book PDF
acceptance result is claimed.

Final production regression: **112 passed, 0 failed, 1 explicitly ignored** in
**528.96 s**, including both selected 5,000-resource PDF cases. The regression
ran against the final behavior; subsequent source edits only wrapped/reordered
exports and formatted the new test call. All launched commands are terminal.
No branch push or new public/full-book PDF acceptance result is claimed.

### Footnote definition labels join measured fragment height

Prepared body flow now retains one opaque definition-marker binding per actual
definition. The binding identifies the first paintable local item and its real
content-relative baseline. Paragraph baselines come from selected lines; vector
baselines include the actual viewport top offset. Rasters and blocks without a
baseline align the label's font ascender to the content top.

The existing list-marker metric calculation is shared. Each label's actual font
ascender/descender expands the item's leading/trailing as necessary. Multiple
labels on one item take maxima rather than adding overlapping extents. Footnote
boundary selection consumes this measured height. A selection exposes its label
only when it contains the bound item; leading forced breaks do not consume the
label, and continuation fragments do not repeat it. Definitions with no paintable
item report `EmptyFootnote` at the definition owner. Each retained binding adds
one record, preserving exact/one-short budget coverage. No page or paint receipt
is issued by this binding.

Final focused footnote tests: **15 passed, 0 failed, 1.37 s**;
`/private/tmp/typaxis-footnote-metrics-final.log`. They cover real paragraph/vector/
list/raster baselines, a 64 pt definition label, shared first-vector extents, exact
consumed-height capacity and a one-unit shortage, continuation ownership, forced
breaks, empty-definition rejection and retained-record boundaries. Initial test
fixtures were corrected to keep the enlarged font local to footnotes and give the
new vector-containing list its true source span.

Commands:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_list --locked
```
List regression log: `/private/tmp/typaxis-footnote-metrics-list-regression.log`.
Reference-page coupling, inter-definition spacing, page reservation/placement,
structure and public paint remain incomplete, as do original full-book/Harano/
scale and both-host acceptance. No new public or full-book PDF result is claimed.

List regression: **11 passed, 0 failed, 2.76 s**, including nested-label height
sharing, continuation pages, real vector/raster baselines, PDF assembly and
independent PDF probes. No functional changes followed these test runs. All
launched test commands are terminal; no branch push is claimed.

### Footnote references bind to actual measured item ranges

Prepared body flow retains one source-borrowing reference binding per actual
occurrence. Both endpoint clusters are checked against their selected paragraph
lines and mapped to local body/definition item indices, including preceding forced
breaks and vector/list content. Repeated targets retain distinct occurrence owners.
The join advances through actual items once and needs no per-paragraph scratch
table. One retained record per occurrence joins the existing exact/one-short
resource-budget tests.

Verified monotonic scope and endpoint ordering supports binary searches for
references intersecting an item range. The query returns a borrowed slice and
grants no page-selection permission. Footnote selections expose occurrences only
from their actual selected range, including none for empty forced-break fragments.

Focused verification: **17 passed, 0 failed, 1.27 s**;
`/private/tmp/typaxis-footnote-reference-items-closure.log`. Coverage includes
wrapped body text with multi-digit "10", repeated targets, every local range in
the multi-line fixture, leading/intermediate forced breaks, definition scopes,
real vector/list offsets and selection occurrence preservation. The nested-reference
fixture also asserts that the current tagged profile rejects it with
`UnsupportedSemantic`; its common-flow test uses the actual lower navigation
profile, host font/vector admission, shaping and selection. Existing tagged test
helpers retain their mandatory accessibility check. The first nested fixture was
corrected to use zero-width source spans at actual forced-break boundaries.

Commands:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked
```
Common-driver regression log: `/private/tmp/typaxis-footnote-reference-items-common.log`.
Deduplicated reservations, reference dependency/page state, joint body/footnote
selection, placement, public PDF and original full-book/Harano/scale/both-host
acceptance remain incomplete. No public/full-book PDF result is claimed.

Common-driver regression: **2 passed, 0 failed, 1 explicitly ignored** in
**0.65 s**. The saved-job diagnostic remains an explicit opt-in test. No
functional edits followed final verification. All launched commands are terminal;
no branch push or new public/full-book PDF result is claimed.

### Branchable footnote demand state and measured region selection

A demand search now shares the existing content search's cumulative record/work
budget. Immutable snapshots retain unreferenced/pending/complete definitions,
first reference owners and first-demand queue order. Repeated or completed targets
are not queued again. Actual selected nested occurrences update the next state;
cycles terminate through already-seen status and finite content cursors. The
current tagged-profile rejection of nested references remains explicitly tested.

Selections bind their issuing search and source snapshot. Cross-owner, cross-branch
and stale-position reuse is rejected. Re-evaluation from an earlier snapshot is
allowed without changing it or refunding records/work. Snapshot copies, queue
retention/moves, occurrence visits and binary-search comparison upper bounds join
the same budget. Owner/state IDs are in-process guards, not artifact content.

Region selection consumes pending definitions in order, with actual marker-inclusive
heights and inter-definition after/before spacing. It records region-relative
offsets and the next demand state. Overflow/forced breaks end the region; leading
empty forced fragments advance once, and a trailing forced break retains its owner
even when all demanded content is complete. Partial selection keeps unmet demands
visible instead of declaring the associated body candidate acceptable.

Final focused regression: **23 passed, 0 failed, 1.29 s**;
`/private/tmp/typaxis-footnote-demand-final.log`. Six new tests cover deduplication
and demand order, branch/owner/stale-selection rejection, failed/dropped attempts,
forced boundaries and complete continuation coverage, nested cycles, actual
inter-definition spacing, exact capacity / one-unit shortage and exact shared
record/work budgets / one-unit shortage (including region selection).

Command:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
```
The earlier 17-test baseline, four state tests and six state/region tests also
finished successfully. No functional changes followed final verification.
These states and regions still do not prove body-boundary legality, same-page
first-reference fit, collision freedom, final placement or public paint. Joint
body/footnote page selection and all original full-book/Harano/scale/both-host
acceptance requirements remain open. No public/full-book PDF result or branch
push is claimed.

### Minimum-preserving footnote allocation and simultaneous body fit

The existing footnote policy requires each pending definition's minimum legal
fragment before distributing extra capacity. A new required-region selector now
implements that rule over real measured boundaries and authored inter-definition
spacing. It reserves suffix minima, filters candidates to preserve later starts,
and recomputes their costs for each actual remaining capacity. Multiple definitions
can continue independently in queue order. A hard forced boundary cannot precede
another selected definition in the same region. The prior partial-region selector
remains distinct and does not claim all requirements fit.

The shared boundary kernel has an internal mode that retains earlier legal cuts
even when the terminal boundary fits. Ordinary body/footnote behavior is unchanged.
All attempted candidate records and visited work retain the same cumulative budget.

A body-footnote candidate now measures an actual local body range, checks keep and
forced boundaries and complete reference ranges, derives collision-free capacity
from the declared rectangles, and reserves the existing **1 pt separator band**.
Positive footnote content is bottom-aligned in its declared maximum region;
horizontally disjoint/above-body regions remain independent. The result binds its
source snapshot, body range/height, actual reservation, content selection and next
demand state. Newly encountered nested demands that have not started cause a
non-fit; the tagged-profile nested-reference gate remains unchanged.

Final footnote regression: **27 passed, 0 failed, 1.76 s**;
`/private/tmp/typaxis-footnote-joint-verified.log`. New coverage includes two long
definitions both starting before extras, exact minimum capacity / one-unit shortage,
independent continuations, separator-inclusive exact geometry, two-body-line
collision versus a fitting one-line cut, disjoint x ranges, above-body regions, keep cuts, forced
obstructions and exact joint record/work budgets / one-unit shortage. Review also
corrected retained candidate capacity/cost consistency after suffix reservation.

Shared boundary regression: **7 passed, 0 failed, 0.01 s**;
`/private/tmp/typaxis-footnote-joint-boundary-regression.log`.
Commands:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-pagination \
  production_body::page_breaks --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked
```
Common-driver log: `/private/tmp/typaxis-footnote-joint-common-regression.log`.
This local fit does not certify a sequence of body pages or rank body alternatives.
Page/reflow limits, stable global page closure, complete fragment placement, public
paint and all original full-book/Harano/scale/both-host gates remain incomplete.
No public/full-book PDF result or branch push is claimed.

Common-driver regression: **2 passed, 0 failed, 1 explicitly ignored** in
**0.82 s**. All launched commands are terminal. No implementation changes
followed final verification; the last added fixture exercised the above-body
geometry branch. No branch push or new public/full-book PDF result is claimed.

### Owned joint body/footnote page search

Implemented `begin_pages` / `select_page` (design §14.31). An opaque page state
owns the body cursor, page index and demand snapshot. Every legal body boundary
is tested for actual simultaneous fit; deterministic common body cost/source
order selects a candidate. Only its state is forked into the next page, with
snapshot/work charges. Search identity, page limits, per-selection reflow limits,
lookback and cumulative budgets remain enforced. Incoming notes can continue on
pages with no body advancement. Leading/consecutive/trailing forced body breaks
preserve blank pages; keep across a forced break fails before selection.

Verification:
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-pagination --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked
```
Final footnote regression **31 passed, 0 failed, 1.82 s**, log
`/private/tmp/typaxis-joint-pages-verified.log`. Common driver **2 passed,
0 failed, 1 explicitly ignored, 0.62 s**, log
`/private/tmp/typaxis-joint-pages-common.log`. Check passed in 7.23 s.
Tests cover selection of a fitting shorter body candidate, owned next-page
continuity, branch retries, rejection of another search, page/reflow caps,
footnote-only continuations, consecutive/trailing blank pages, and exact/one-short
record/work budgets including the selected snapshot. Initial test failures were
fixture JSON path/source-span key mistakes; corrected before final regression.

No public/full-book PDF or branch push is claimed. This is local page selection
using existing body boundary costs, not a complete global pagination policy or
stability/paint receipt. Physical placement, convergence, public writer, original
full-book, Harano, scale and both-host acceptance gates remain incomplete.

### Selected joint page content and marker placement

Implemented design §14.32. `place_page_content` accepts the same search's actual
page selection and returns borrowed-selection geometry retaining definition-local
versus body-local item indices. Footnote origins include the actual reservation's
separator band and selected fragment offsets. Exact consumed heights are checked.
Ordinary body and joint placement now share `place_flow_item` for paragraph
baselines, vector viewports/baselines and raster bounds.

List markers use existing measured bindings and placement, locating their original
stream item with a charged binary search (the preparation pass validates ordering).
Footnote definition markers use actual shape advance, shared marker column and
font ascender/descender, and are placed only when their bound first paint item is
selected. Continuation pages do not repeat them. Result records and work consume
the same cumulative search budget; retries never refund charges.

Verification commands (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-pagination --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_body_raster --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_list --locked
```
Final footnote regression: **33 passed, 0 failed, 1.90 s**,
`/private/tmp/typaxis-joint-placement-verified.log`. Tests cover real body/definition
origins, scopes, item spacing, source/owner identity, list/definition marker
placement, continuation marker suppression, wrong-search rejection, repeat
placement, and exact/one-short cumulative work/record budgets including markers.

Shared geometry regression: common driver **2 passed, 1 explicitly ignored,
0.65 s**, raster **3 passed, 1.02 s**, lists **11 passed, 0.77 s**. Logs are
`/private/tmp/typaxis-joint-placement-{common,raster,lists}.log`. These regressions
ran after the shared helper extraction; subsequent edits added only joint marker
placement/tests. Marker check passed in 1.41 s. No public/full-book PDF or branch
push is claimed.

Still required: stable complete page sequence, math terminal binding, separator
paint and complete display/tag/navigation/manifest/public writer integration,
plus original full-book, Harano, scale and both-host acceptance. Physical fragment
coordinates alone do not satisfy those gates.

### Complete joint page selection and placement sequences

Implemented design §14.33. `select_pages` owns all pages from a fresh body start
to body/demand/required-blank-page completion. Each page derives only from its
predecessor's owned next state. The loop checks forward progress, page continuity
and terminal completion; a non-fitting intermediate page returns `JointPageNoFit`
instead of a partial completed sequence. Page, work, lookback and record limits
remain cumulative. `place_pages_content` borrows a verified complete sequence and
returns only after every page's content and markers have been placed successfully.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-pagination --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked
```
Check passed in **1.61 s**. Final footnote regression **37 passed, 0 failed,
2.21 s**, `/private/tmp/typaxis-joint-sequence-verified.log`. New sequence tests
cover body/definition continuity to each actual source end, markers exactly once,
all-page borrowed geometry, foreign-owner rejection, no-fit and partial page-cap
failure, exact/one-short cumulative record/work budgets, and preservation of
leading/consecutive/trailing blank pages.

The unreferenced-definition fixture initially failed the existing tagged-profile
preflight with `UnsupportedSemantic`. Its low-level omission test now uses the
existing navigation-profile helper, which also asserts that tagged preflight
still rejects the fixture. This does not widen public tagged PDF support.

Common-driver regression **2 passed, 0 failed, 1 explicitly ignored**;
`/private/tmp/typaxis-joint-sequence-common.log`. No public/full-book PDF or branch
push is claimed. Page-sequence stability, dynamic-reference convergence, terminal
binding, separator paint, complete display/tag/navigation/manifest/public writer,
and original full-book/Harano/scale/both-host acceptance remain incomplete.

### Repeated joint page selection and physical-placement equality

Implemented design §14.34. `select_stable_pages` performs at least two complete
page searches from the same immutable measured flow and physically places both
results. It issues an owned `ProductionBodyFootnoteStablePages` only after exact
comparison of body ranges/heights, reservation geometry, forced breaks, next body
positions, demanded-definition order/status/first-reference/continuation cursors,
footnote fragment boundaries/capacities/cost choices, and all content/list/footnote
marker geometry. Snapshot identities are excluded from content equality, while
both source sequences are authenticated to the same owner/flow first. Every
comparison record consumes work; every pass/search/placement shares the existing
cumulative budget. No partial result is exposed on any failure.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-pagination --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked
```
Check passed **3.95 s**. Focused stability tests **2 passed, 0 failed, 0.90 s**,
`/private/tmp/typaxis-joint-stability-tests.log`. Final footnote regression
**39 passed, 0 failed, 2.53 s**, `/private/tmp/typaxis-joint-stability-final.log`.
Tests exercise actual joint/long-definition/vector-list fixtures, repeated complete
search and placement, a minimum of two passes, exact/one-short record/work budgets
including comparison, cumulative retries, foreign-owner rejection, and forced
blank pages. No-fit/page-limit cases also reject stable-result issuance.
Common driver **2 passed, 0 failed, 1 explicitly ignored**;
`/private/tmp/typaxis-joint-stability-common.log`.

This proves repeated page equality only for one already measured immutable flow.
It is not dynamic reference/line convergence, final math terminal or separator
paint closure, complete display/tag/navigation/public writer/manifest, or any
original full-book/Harano/scale/both-host acceptance result. Those gates remain
incomplete. No public/full-book PDF or branch push is claimed.

### Explicit footnote separator ink geometry

Implemented design §14.35. The existing footnote painter's 0.5 pt stroke and
0.25 pt center offset now use shared layout constants alongside the existing
1 pt band. Joint placed pages retain the exact full-width ink rectangle at the
reservation top, charge its record/work, and compare it during repeated-page
stability checks. Empty forced footnote fragments/blank pages have no separator;
continuations containing actual content do. Existing painter dimensions are
unchanged. This does not yet emit the new joint pathway's PDF commands.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-display-list \
  footnote --locked
```
Final footnote regression **40 passed, 0 failed, 2.89 s**,
`/private/tmp/typaxis-joint-separator-final.log`. Assertions cover reservation
origin/width, exact stroke/center constants, continuation separators, absence on
blank pages and empty forced footnote selections, repeated placement/stability,
and exact/one-short cumulative work/record budgets including separator geometry.
Display-list filter **1 passed, 0 failed**, log
`/private/tmp/typaxis-joint-separator-display.log`; this existing test covers marker
clusters, not separator rendering. Painter constant reuse was compiled and
inspected; no new independent-render/PDF acceptance is claimed.

Public joint display/terminal/tag/navigation/manifest closure, dynamic-reference
convergence, and original full-book/Harano/scale/both-host acceptance remain
incomplete. No public/full-book PDF or branch push is claimed.

### Stable joint-page block math terminal closure

Implemented design §14.36. `finalize_page_math` authenticates the same search's
stable sequence, its exact borrowed geometry, limits, registry and epoch. It uses
the extracted common `consume_selected_fragment` for real block/viewport/baseline
and equation-number validation, consumes each selected block in the registry's
terminal ledger, and requires complete finish/verify and number counts. The result
borrows the physical geometry and retains terminal receipts and equation-number
placements indexed into the flattened page/content stream. Ordinary body terminal
validation uses the same helper; no dummy terminal or unplaced-flow exception was
introduced.

Records and work use the same cumulative search budget. Terminal spool reservations
are checked before ledger allocation, accumulate across attempts, and bound the
measured retained canonical bytes. This is terminal-phase accounting, not complete
pipeline spool/publication closure.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-pagination --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_body_equation_numbers --locked
```
Check **2.94 s**. Focused terminal tests **2 passed, 1.55 s**. Final footnote
regression **42 passed, 0 failed, 2.64 s**,
`/private/tmp/typaxis-joint-terminal-closure.log`. Coverage includes real body and
footnote formula blocks, empty block registry, numbered body and numbered footnote
formulas, stable-result/geometry mismatch, foreign search, and exact/one-short
record/work/spool budgets with cumulative retries. The new numbered-footnote
fixture preserves its declared styled container and uses an explicitly wide
footnote region for the number plus definition-marker columns. Its initial missing
inherited text style and insufficient-width failures were fixture corrections;
no fallback font or formula scaling was added.
Common driver **2 passed, 1 explicitly ignored, 0.64 s**, log
`/private/tmp/typaxis-joint-terminal-common.log`; equation-number regression
**3 passed, 0.31 s**, `/private/tmp/typaxis-joint-terminal-numbers.log`.

Public joint display/tag/navigation/PDF/manifest integration, dynamic reference
convergence, original full-book/Harano/scale/both-host acceptance remain incomplete.
No public/full-book PDF or branch push is claimed.

### Internal display projection inputs separated from ordinary-body ownership

Implemented design §14.37. A private borrowed `DisplayInput` now supplies the
existing glyph/inline-anchor/vector/raster/list/equation projection. The public
ordinary-body builder still authenticates and retains its existing selected
layout; no public arbitrary-geometry constructor was added. Shape provenance,
resource checks, record charging and fingerprint rules remain in the common
projection. Equation numbers are located by their selected fragment index and
then checked against parent owner, avoiding a source-owner ordering assumption
for the forthcoming demand-ordered footnote adapter.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-display-list --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_body_display --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_body_equation_numbers --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_list --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked
```
Check **4.51 s**. Display **2 passed, 0.17 s**; equation numbers **3 passed**;
lists **11 passed**; common driver **2 passed, 1 explicitly ignored**. Logs:
`/private/tmp/typaxis-display-input-{tests,numbers,lists,common}.log`.
These are regressions of the existing entry point, not evidence of joint footnote
paint. The authenticated joint adapter, footnote-number/separator draws and
complete tags/navigation/PDF/manifest remain pending, as do dynamic reference
convergence and original full-book/Harano/scale/both-host acceptance. No public
full-book PDF or branch push is claimed.

### Authenticated joint body/footnote display projection

Implemented design §14.38. `build_production_footnote_display` borrows completed
stable-page math terminals, authenticates admitted resources/limits and retains
the source binding. The shared display input now separates source lifetimes from
temporary page-fragment views and supports a checked global fragment offset.
Text, inline anchors, vectors, raster figures, list markers and equation numbers
use the existing projection; footnote and list labels share generated-glyph paint
logic. Separator records retain page/ink/before-draw placement. No arbitrary
public geometry constructor or substitute PDF receipt was introduced.

The incremental fingerprint binds source receipts and actual fragment/marker/
separator geometry without a document-sized hash buffer. Per-invocation record
accounting starts at the authenticated terminal charge and includes temporary
fragment/projection and retained output allocations. It does not itself govern
lifetimes/retries across independent display-building invocations; the complete
pipeline owner remains to be connected.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-display-list --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_list --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_body_equation_numbers --locked
```
Check **4.93 s**. Focused display tests **2 passed, 0.92 s**. Final footnote
regression **44 passed, 0 failed, 6.25 s**,
`/private/tmp/typaxis-joint-display-verified.log`. Coverage includes real body and
footnote formula/number/list geometry, generated definition text/glyph counts,
global fragment/page mapping, reversed demand order, footnote-only continuations,
raster projection, separator insertion before actual footnote paint, repeatable
fingerprints and exact/one-short preceding-plus-projection record budgets.
List regression **11 passed, 1.90 s**; common driver **2 passed, 1 explicitly
ignored, 1.25 s**; equation numbers **3 passed, 0.48 s**. Logs:
`/private/tmp/typaxis-joint-display-{lists,common,numbers}.log`.

This is joint display data, not yet emitted/tagged joint PDF content. Separator
commands/artifact classification, footnote structure and links, font usage/content/
objects/manifest/public writer, complete pipeline budgets, dynamic-reference
convergence, and original full-book/Harano/scale/both-host acceptance remain
incomplete. No new public/full-book PDF or branch push is claimed.

### Shared structure grouping for joint body/footnote display

Implemented design §14.39. `build_production_footnote_structure` authenticates the
joint display and existing navigation/accessibility authorizations, uses the
existing v2 registry, and projects actual draws into dense page MCIDs and node MCR
groups. Ordinary-body structure now uses the same projection. Footnote-generated
labels resolve through the existing FootnoteLabel slot and actual generated key/
canonical text, retaining Note/Reference parentage. Label coverage checks reject
missing/duplicated/noncontiguous ranges; definition/list labels require one group,
while reference labels may span groups only within one page. Separator records
are exposed as artifacts and do not receive MCIDs.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-display-list --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_list --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_body_equation_numbers --locked
```
Check **5.52 s**. Focused structure tests **2 passed, 1.13 s**. Final footnote
regression **46 passed, 0 failed, 5.78 s**,
`/private/tmp/typaxis-joint-structure-verified.log`. Tests cover real formula/list/
numbered-definition fixtures, continued notes, reversed demand order, multi-digit
labels, Note/Reference parentage, complete draw coverage, dense MCIDs, artifact
separators, exact/one-short cumulative record limits and rejection of another
display instance. Common/list/equation regression logs:
`/private/tmp/typaxis-joint-structure-{common,lists,numbers}.log`.
All passed: common **2 passed, 1 explicitly ignored**; lists **11 passed**;
equation numbers **3 passed**.

No joint marked-content/ParentTree/PDF/annotation/manifest output is claimed.
Those integrations, logical-versus-physical reading-order/extraction checks,
complete pipeline accounting, dynamic references and original full-book/Harano/
scale/both-host acceptance remain incomplete. No branch push is claimed.

### Joint font usage/subset planning after structure validation

Implemented design §14.40. `finalize_production_footnote_fonts` authenticates and
borrows the joint structure/display, then uses the extracted common font
projection also used by ordinary body finalization. Actual text face/span/string/
glyph usage covers body, footnotes, reference/definition/list labels and equation
numbers. Non-text draws have no usage slot. The result maps draw indices to
frozen font/cluster/CID plans and rejects verification with another structure
instance. It starts with structure record charges and retained structure plus
math-terminal spool bytes before charging font work/copies/subset bytes.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-resources --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_footnote --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_common_driver --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_body_objects_preserve_tt_ttc_cff_font_programs_and_unicode --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  production_body_display --locked
```
Check **3.62 s**. Final footnote regression **48 passed, 0 failed, 3.50 s**,
`/private/tmp/typaxis-joint-font-final.log`. New tests compare actual used face sets,
source spans, exact strings, glyph IDs and CID counts for real list/formula/
numbered-footnote/multi-digit fixtures; verify nonempty subsets and absent
non-text plans; reject foreign structure identity; and exercise exact/one-short
record and spool budgets including preceding stages. An initial test incorrectly
assumed one face; the fixture actually uses two. The test now requires exact set
equality instead of changing the implementation or substituting a font.
Regression logs: `/private/tmp/typaxis-joint-font-{common,formats,body}.log`.
Common **2 passed, 1 explicitly ignored**; existing TT/TTC/CFF object/Unicode test
**1 passed**; body display **2 passed**. The format regression covers the existing
small ordinary-body path, not original Harano or a complete joint PDF.

Joint PDF text/content, marked content, ParentTree/font objects, links, manifest
and public writer remain pending, together with full pipeline retry/retention
accounting, dynamic references and original full-book/Harano/scale/both-host
acceptance. No new public/full-book PDF or branch push is claimed.

## Joint body/footnote PDF text contribution

Implemented §14.41: authenticated joint font plans now feed the shared existing
text encoder. Each retained paint binds its draw/page/font instance and uses the
actual selected coordinates and frozen CIDs. Standalone ActualText wrappers and
the inner commands are separately accessible for subsequent marked-content
assembly. Another font-plan instance is rejected. Prior font records/spool are
included before allocating text paint records and retaining command bytes.

Local verification:
- `CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build cargo check --manifest-path workspace/Cargo.toml -p typaxis-pdf`: passed, 4.40 s.
- Footnote regression before the additional output-limit boundary cases:
  49 passed, 0 failed, 3.41 s, `/private/tmp/typaxis-joint-text-final.log`.
- Tests cover real body/footnote/list/equation/multi-digit draws, exact font-plan
  identity, paint ordering, parsed PDF coordinates/CIDs and ActualText wrapper
  selection; exact and one-short record/spool/output budgets traverse all
  preceding phases. An initial string comparison incorrectly expected the
  floating-point display's shorter decimal spelling; the corrected test parses
  emitted numbers and compares fixed-point coordinates and ordered CIDs.

The joint page/vector/raster/separator stream, marked content, object graph,
links, manifest and public writer still require integration. Full pipeline
retry/retention budgeting and dynamic references remain pending. This change
does not establish original full-book/Harano/scale/both-host acceptance or
produce a newly authorized public PDF.

Final regression (terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli production_ --locked
```
**148 passed, 0 failed, 1 explicitly ignored, 240.88 s**,
`/private/tmp/typaxis-joint-text-regressions.log`. This includes the final
output-budget cases, ordinary common driver, font object/Unicode and body
display tests, and both existing 5,000-SVG page-content tests (alias sharing
and distinct Forms). The ignored saved-job test requires explicit input/output
paths. The 5,000-SVG cases validate the existing ordinary-body content path,
not original-book or complete joint-footnote PDF acceptance. No branch push.

## Joint vector/raster plans and complete page content

Implemented §14.42–14.43. Joint font plans feed the same Form/alias usage
projection as ordinary body display. Each vector retains its image ID, draw
ordinal, page, matrix, scale, currentColor and semantic hook; content sharing
does not merge occurrences. The joint vector PDF entry verifies that its
preceding text contribution belongs to the same font-plan instance and derives
remaining spool from the actual preceding bytes.

Joint raster plans use the existing PNG/alpha/Flate and admitted JPEG pipeline.
The common page builder combines actual text/vector/raster draws in order,
retains selected empty pages and applies one page-root Y flip. A 0.5pt black
butt separator is emitted at its selected geometry immediately before the first
footnote draw, in an independent Artifact scope. Artifact byte ranges and
insertion draw indices remain available separately for subsequent marked
content; no MCID is assigned to a separator.

Verification completed before broad regression:
- Resource check 3.17 s; PDF vector check 2.84 s; integrated PDF check 3.55 s.
- Focused vector tests: 2 passed, 1.47 s.
- Focused page-content filter: 4 passed, 2.72 s.
- Final footnote regression after adding a real PNG inside a footnote definition
  and numeric page/raster matrix assertions: **53 passed, 0 failed, 9.37 s**,
  `/private/tmp/typaxis-joint-content-footnotes.log`.
- Existing Form tests: **4 passed, 0 failed, 0.14 s**,
  `/private/tmp/typaxis-joint-content-form-regression.log`.

Commands use `CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build` and
`--manifest-path workspace/Cargo.toml --locked`:
```sh
cargo check --manifest-path workspace/Cargo.toml -p typaxis-resources --locked
cargo check --manifest-path workspace/Cargo.toml -p typaxis-pdf --locked
cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_footnote_vectors --locked
cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_footnote_page_content --locked
cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_footnote_ --locked
cargo test --manifest-path workspace/Cargo.toml -p typaxis-resources safe_vector_v2::tests --locked
```

New tests cover actual vector Form counts/usage identities and deterministic
contribution fingerprints, foreign font/text owners, exact/one-short records
and retained spool; integrated content covers body and definition math, equation
numbers, continuing notes, leading/consecutive/trailing blank pages, PNG/JPEG,
PNG inside a definition, actual text/vector/raster command selection, page/raster
matrix values and Artifact line geometry/order. Final page-content records,
retained spool and output byte limits include all preceding stages in each run.

This is a content-stream contribution, not a complete public PDF. Marked
content, ParentTree/font/image objects, links, manifest and the public writer
still need connection. Complete IR/canonical allocation lifetimes and
cross-retry/branch budgets remain open, as do dynamic references and original
full-book/Harano/scale/both-host acceptance. No public receipt or push is claimed.

Final broad regression (terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_ --locked
```
**152 passed, 0 failed, 1 explicitly ignored, 378.18 s**,
`/private/tmp/typaxis-joint-content-final.log`. Both existing 5,000-SVG
page-content cases (one shared Form for aliases and distinct Forms) passed,
alongside common-driver, ordinary body/raster/object/Unicode/structure tests.
The ignored saved-job test requires explicit input/output paths. These
regressions do not replace original-book performance or independent full-PDF
acceptance. All processes from this implementation step are terminal.

## Joint body/footnote marked content

Implemented §14.44. The authenticated joint page contribution feeds the shared
marked-content projector. Each selected structure group receives its actual
role, dense page MCID and language; text ActualText contains only the selected
draw strings. Formula semantic anchors retain actual viewport/baseline values.
Separators are copied from their exact source byte ranges outside all MCID and
ActualText scopes, before the associated group. An artifact splitting a group
or left unconsumed is rejected. Blank pages remain in the result.

The result borrows its exact page-content owner and obtains the same structure
through the existing font plan. The joint path already includes structure
records/spool before font finalization, so marked-content construction starts
from content charges without repeating the ordinary-body parallel-branch merge.
Pages, nonpainting formula anchors and new marked bytes are then charged.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_footnote_ --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_ --locked -- --skip production_body_page_content_places_5000
```
Footnotes before the final multi-digit addition: **54 passed, 0 failed, 8.71 s**,
`/private/tmp/typaxis-joint-marked-tests.log`.
Final regression: **151 passed, 0 failed, 1 explicitly ignored, 16.32 s**,
`/private/tmp/typaxis-joint-marked-final.log`. This covers the final multi-digit
footnote case, actual reference/definition/equation text, scope balancing,
artifact exclusion from structure scopes, exact ordered ActualText strings,
formula anchor geometry, owner identity, and exact/one-short records/spool/output
budgets. Existing common driver, structure, object/Unicode and assembly tests
also pass. The two 5,000-SVG unmarked page-content cases were explicitly excluded
from this run; their shared code was not changed in this step and their previous
run remains recorded above. The saved-job test remains explicitly ignored.

No completed public/full-book PDF is issued. Navigation including footnote
destinations/annotations, final structure/ParentTree/font/image objects, manifest
and the public writer remain to connect, together with dynamic references,
complete allocation/retry budgeting and original full-book/Harano/scale/both-host
acceptance. No push.

## Footnote definition destinations and reference hit areas

Implemented §14.45. A new authenticated footnote-reference navigation plan
uses the actual terminal-owned definition/reference mapping and generated
FootnoteLabel groups. Definition destinations retain source definition indices
while reference links retain selected paint/page order. Each Reference maps
to its exact definition index; repeated references and multiple digits remain
distinct source occurrences. The first definition fragment, its number and
all associated paints contribute to the destination bounds, including large
list markers, formulas and figures. Continuations do not create another
definition destination. Hit rectangles cover only actual reference-number
draws, joined within the same fragment.

The result stores Note/Reference structure nodes, page/fragment coordinates,
page link ranges, exact structure identity and a deterministic fingerprint.
Missing/duplicate destinations, unplaced references, invalid geometry/order
and foreign structure/admission/limits are rejected. Navigation records start
from the caller's retained marked-content charge, never below structure charge;
temporary indexes, retained destinations and links are included.

Verification (all terminal; target directory
`/private/tmp/typaxis-vmb-book-build`, manifest `workspace/Cargo.toml`):
- `cargo check -p typaxis-display-list --locked`: passed, 7.20 s.
- `cargo test -p typaxis-cli --bin typaxis production_footnote_ --locked`:
  **55 passed, 0 failed, 9.68 s** before the final reversed-order/first-row
  additions; `/private/tmp/typaxis-footnote-nav-tests.log`.
- `cargo test -p typaxis-cli --bin typaxis production_ --locked -- --skip production_body_page_content_places_5000`:
  **152 passed, 0 failed, 1 explicitly ignored, 15.02 s**;
  `/private/tmp/typaxis-footnote-nav-final.log`. Reversed reference/definition
  order, repeated references, multiple digits, real formula/raster definitions,
  continuing notes, exact/one-short retained record budgets, identity rejection
  and existing ordinary navigation/structure/object/assembly regressions pass.
- After adding a definition starting with a large list marker and checking
  that each destination contains all paints of its first fragment:
  `cargo test -p typaxis-cli --bin typaxis production_footnote_page_content_combines --locked`,
  **1 passed, 0 failed, 2.58 s**, `/private/tmp/typaxis-footnote-nav-first-row.log`.

The two unchanged 5,000-SVG unmarked-content cases were explicitly excluded
from this regression; their prior completed evidence remains above. The
saved-job test remains explicitly ignored. This dedicated reference plan is
not complete joint navigation: ordinary anchors/URI/outline, PDF annotation
objects/StructParent, destination serialization and needed return links still
require integration. No public PDF receipt, original-book/Harano/both-host
acceptance or push is claimed.

## Joint ordinary and footnote navigation

Implemented §14.46. The ordinary anchor/internal-link/URI/outline projection
now accepts actual selected draws, inline anchors and a fragment iterator.
The ordinary body path retains its existing projection; the joint path supplies
the complete body/footnote geometry without allocating another fragment copy.
Ancestor anchor positions, per-node and per-page link indices, physical link
rectangles and outline topology use the same source-grounded algorithm.

The joint owner retains ordinary navigation plus the separately typed footnote
reference plan. It adds ordinary navigation records before constructing the
footnote plan and exposes the combined additional charge. Both branches bind
to the same structure, and the combined fingerprint includes both fingerprints.
Ordinary anchor indices and footnote definition indices remain distinct.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-display-list --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_footnote_ --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_ --locked -- --skip production_body_page_content_places_5000
```
Check **3.62 s**. Initial footnote suite **56 passed, 0 failed, 7.72 s**,
`/private/tmp/typaxis-joint-navigation-tests.log`. Final regression after adding
an internal body link targeting an anchor inside a footnote:
**153 passed, 0 failed, 1 explicitly ignored, 15.54 s**,
`/private/tmp/typaxis-joint-navigation-final.log`.

Coverage includes body-to-footnote internal navigation, a URI link inside the
definition, multi-page ordinary links, source anchor names/owners, physical
inline-anchor coordinates, outline topology, node/page link indices, inherited
record charges, exact/one-short combined budgets and foreign structure identity.
Existing ordinary navigation/object/assembly tests pass. The two unchanged
5,000-SVG unmarked-content cases were explicitly excluded; the saved-job test
remains explicitly ignored.

This is a navigation plan, not PDF annotations or a public PDF receipt.
Merging annotation order and StructParent ownership, serializing destinations
and outlines, needed return links, final objects/manifest/public writer,
dynamic references and full allocation/retry accounting remain open, along
with original full-book/Harano/scale/both-host acceptance. No push.

## Joint PDF annotation objects and binding indices

Implemented §14.47. The marked-content owner now produces actual typed
LinkAnnotation objects for both ordinary links and footnote references.
Annotations are ordered by selected page/fragment/source owner, with retained
source-kind/index, structure node, page and StructParent key. Per-page ranges
and per-node annotation indices are available for the later Annots/ParentTree/
OBJR owner.

Rect and direct footnote XYZ destinations use actual PDF-coordinate conversion.
Ordinary named destinations keep their source names, URI actions keep original
validated bytes, and footnote destinations directly reference their definition
page without inventing names in the ordinary anchor namespace. Contents comes
from the ordinary accessible name or source footnote number. Objects retain
unresolved typed Page references. Records and spool include prior marked/
navigation state, sorting/index records and the shared object's dictionary/
chunk accounting.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-pdf --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_footnote_ --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_ --locked -- --skip production_body_page_content_places_5000
```
Check **9.29 s**. Initial footnote tests **57 passed, 0 failed, 5.28 s**,
`/private/tmp/typaxis-footnote-annotations-tests.log`.
Final regression **155 passed, 0 failed, 1 explicitly ignored, 11.39 s**,
`/private/tmp/typaxis-footnote-annotations-verified.log`.

Tests inspect actual dictionary bytes and typed references: exact rectangle and
XYZ coordinates, URI/name/Contents encodings, annotation order, source mapping,
page references, dense StructParent keys, node/page indices, foreign marked
owners and exact/one-short record/spool limits. A conflicting enclosing Link
test initially attempted to reach annotations, but the existing tagged-profile
preflight correctly rejected it as UnsupportedSemantic; the final test verifies
that earlier boundary. The defensive annotation guard uses a single
parent-before-child pass with charged temporary storage.

The two unchanged 5,000-SVG cases were explicitly excluded; the saved-job test
remains explicitly ignored. This contribution still needs integration with
ParentTree/OBJR/page Annots and all final objects, named destinations/outlines,
needed return links, manifest and public writer. Full object-budget closure,
dynamic references, allocation/retry lifetimes and original full-book/Harano/
scale/both-host acceptance remain open. No public PDF receipt or push.

## Joint ParentTree and structure objects

Implemented §14.48. The exact annotation owner feeds the same structure-object
projector as ordinary body output. The contribution includes StructTreeRoot,
ParentTree, every structure node and optional IDTree. Page arrays follow actual
MCID order; annotation keys map to the retained Link/Reference structure nodes.
Each node retains registry children and attributes, actual MCRs and matching
OBJRs pointing to the annotation's page and LinkAnnotation role. Artifacts
are absent from the structure tree.

Before allocation, the combined retained annotation and new structure object
count is checked. Records/spool start from the complete annotation contribution.
The opaque result borrows that exact owner and rejects another annotation
instance. Page references remain typed and unresolved until final assembly.

Verification (all terminal):
```sh
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo check --manifest-path workspace/Cargo.toml -p typaxis-pdf --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_footnote_ --locked
CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build \
  cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_ --locked -- --skip production_body_page_content_places_5000
```
Check **1.08 s**. Footnotes **59 passed, 0 failed, 5.23 s**,
`/private/tmp/typaxis-joint-structure-objects-tests.log`.
Final regression **156 passed, 0 failed, 1 explicitly ignored, 13.15 s**,
`/private/tmp/typaxis-joint-structure-objects-final.log`.

Tests inspect ParentTree reference order and key bytes, ParentTreeNextKey,
all node parent/child/MCR/OBJR references, page/annotation ownership, complete
structure object counts, foreign owner rejection and exact/one-short combined
object/record/spool budgets. Existing ordinary structure/object/assembly
regressions pass. The unchanged 5,000-SVG unmarked-content cases were explicitly
excluded; the saved-job test remains explicitly ignored.

Final page Annots/StructParents, font/image/vector/content object integration,
ordinary destinations/outlines, complete numbering/manifest/public writer,
needed return links, dynamic references and full allocation/retry budgets
remain open, as do original full-book/Harano/scale/both-host acceptance.
No completed public PDF or push is claimed.

## Joint font/media/page resource and navigation objects

Implemented §14.49. The exact structure-object contribution now feeds the
shared font, semantic-anchor, vector/ExtGState, raster/mask, marked PageContent
and PageResources projector. Per-page dictionaries reference actual selected
font/vector/raster plans; programs and image payloads are retained unchanged.
Source pages and semantic anchors must be fully consumed.

The same contribution also emits ordinary destination name trees and outline
objects through the shared navigation-target projector. Name-tree order follows
UTF-16 code units; actual page/XYZ positions and outline parent/sibling roles are
preserved. Every local reference must resolve within this contribution, while
checked Page references remain for final assembly.

The object Builder now includes an explicit prior-object count when admitting
each new object. The joint structure path supplies retained annotation count;
the resource path supplies annotation plus structure counts. Records/spool
continue from the structure result. The new result exposes the complete
retained annotation/structure/resource object count without copying old objects.

Verification completed before final regression:
- PDF resource check **1.22 s**; navigation-target check **2.73 s**.
- Resource footnote suite **60 passed, 0 failed, 6.70 s**,
  `/private/tmp/typaxis-joint-resource-objects-tests.log`.
- After navigation-target integration, footnotes **60 passed, 0 failed, 8.00 s**,
  `/private/tmp/typaxis-joint-resource-targets-tests.log`.

All commands use `CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build`,
`--manifest-path workspace/Cargo.toml` and `--locked`:
```sh
cargo check --manifest-path workspace/Cargo.toml -p typaxis-pdf --locked
cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_footnote_ --locked
cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_ --locked -- --skip production_body_page_content_places_5000
```

Tests inspect actual TT/TTC/CFF1 program bytes and Type0 references, ToUnicode,
PNG/JPEG and mask payloads, marked streams, exact per-page resource reference
sets, local reference closure, destination ordering and outline roles. They
exercise exact/one-short cumulative object, record and spool limits and reject
another structure-object owner. The CFF case is the existing small fixture,
not original Harano or the next public CFF profile.

Final Page Annots/StructParents, page tree/catalog/metadata, absolute numbering/
xref, manifest/public writer, needed return links, dynamic references and
complete allocation/retry budgets remain pending, together with original
full-book/Harano/scale/both-host acceptance. No public receipt or push.

Final regression (terminal): **157 passed, 0 failed, 1 explicitly ignored,
22.48 s**, `/private/tmp/typaxis-joint-resource-objects-final.log`.
Existing ordinary font/object/structure/navigation/assembly and cumulative
budget regressions pass. The unchanged 5,000-SVG unmarked-content tests were
explicitly excluded; the saved-job test remains explicitly ignored. All
processes from this implementation step are terminal.

## 2026-09-08 — Joint body/footnote diagnostic PDF assembly

Implemented §14.50. The common assembler now accepts borrowed object iterators,
source-authenticated metadata/geometry and the selected page annotation ranges.
`assemble_production_footnote_pdf` joins annotations, structure and resource
contributions, counts the complete graph before numbering, resolves every typed
reference, and emits Pages/Catalog/metadata, Page Annots/StructParents, xref and
trailer. It carries the cumulative records/spool and checks final output bytes;
the opaque result rejects a different resource owner. Ordinary assembly uses
the same projector.

Tests cover 16 fixtures (TT/TTC/CFF1, navigation, reversed/repeated/multi-digit
footnotes, continued definitions, large list labels, formula numbers, empty
pages, PNG/JPEG). They inspect every object's bytes after reference substitution,
hash/offset/xref, page resources/content and merged annotation ranges, repeat
bytes, and receipt identity. Exact/one-short records, spool, total objects and
output limits are exercised. No public PDF/UA receipt is asserted.

Validation:
- `cargo check -p typaxis-pdf --locked`: pass, 16.28 s,
  `/private/tmp/typaxis-joint-assembly-check.log`.
- CLI `production_footnote_`: 61 passed, 0 failed, 14.29 s,
  `/private/tmp/typaxis-joint-assembly-tests-2.log`.
- CLI `production_ -- --skip production_body_page_content_places_5000`:
  158 passed, 0 failed, 1 explicitly ignored, 21.18 s,
  `/private/tmp/typaxis-joint-assembly-clean-final.log`.
  Target directory `/private/tmp/typaxis-vmb-book-build`; all commands use
  `--locked` and the workspace manifest. The unchanged 5,000-SVG tests were
  excluded and the saved-job test remains ignored.

Diagnostic PDFs are in `/private/tmp/typaxis-joint-pdf-probes.3oKa2e`.
`inspect.py` ran Poppler 26.08.0 and MuPDF 1.28.2 over 16 PDFs / 33 pages:
80 info/extraction/structure-root/render checks passed without stderr warnings.
`report.json` records hashes and both extracted texts. MuPDF independently
resolved the mixed fixture's four annotations (ordinary destination, two
footnote XYZ destinations, URI) through ParentTree and the owning OBJRs;
`mixed-links.log` records those dictionaries. Visual inspection of the mixed
fixture's first page confirms the separator and formula placement, but these
small/artificial fonts do not prove original-book typography or Harano output.
Poppler and MuPDF extraction order differs in this fixture; this is not a
complete reading-order/accessibility acceptance audit.

The first probe-enabled broad run had 5 machine-test failures because the strict
CLI environment parser correctly rejected `TYPAXIS_FOOTNOTE_PDF_PROBE_DIR`.
It still produced all 16 probes. The clean regression above ran without that
diagnostic variable; the CLI parser was not relaxed.

Manifest/public writer/CLI integration, required return links, dynamic page
references, complete allocation/retry ownership and all original-book/Harano/
scale/both-host gates remain required. This is inspectable diagnostic assembly,
not `VerifiedPdfBytesReceipt`, publication authorization, or completion of doc 28.

Final strengthened budget fixture uses actual footnotes across multiple pages:
`production_footnote_assembly_keeps_complete_graph_and_byte_budgets` passed
1/1, 2.02 s, `/private/tmp/typaxis-joint-assembly-footnote-budget.log`.
All processes from this step are terminal.
