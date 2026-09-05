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
| Shared body/math flow and selected text placement | Additional design §14; fixed text metrics and placeholder page/paint geometry must be replaced before full-book verification |
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
