# VMB book production implementation evidence

Status: In progress. This ledger does not narrow the scope of
[design 28](28-vmb-book-production-compatibility.md). No full-book success or
Harano support is claimed until the corresponding gates have evidence.

| Requirement | Current evidence / remaining work |
| --- | --- |
| ADR-0038 lexical exception | Implemented and covered by the 259-test run below |
| Safe-SVG 2 whitespace, multiple paths, curves, subpaths | Implemented; detailed-error / unchanged real-SVG tests passed in the 259-test run |
| SVG detailed reasons/spans/path/attribute/budget, JSON notes | In progress: fixed context, tag/attribute/path spans, typed CLI code, bounded percent tokens; full reason/budget coverage pending |
| Count/analyze/build internal mismatch diagnostic | Changed to receipt invariant / I9190; verification pending |
| Profile defaults and override precedence | Implemented; config tests and CLI negative boundaries passed |
| Original image/font count diagnostic pointer | Unit and both CLI runner boundary tests passed |
| Resolver cursor and finalized dense image lookup | Existing aggregate/order/admission regressions passed; mixed/5,000 performance evidence pending |
| Real VMB fixtures and provenance ledger | First unchanged chapter SVG + TeX/hash provenance recorded; remaining cases and complete index pending |
| 300–500 chapter and 5,000 placed distinct images / mixed aliases | Pending |
| 8,192 / 8,193 and explicit lower-limit CLI tests | 8,193 and explicit 1,025 rejection passed; 8,192 positive boundary pending |
| Detailed font diagnostics and TTC face list | Pending |
| CID CFF /2, FD-aware evaluator, subset / PDF integration | Pending |
| Vertical tables, cmap 14, IVS shaping/extraction | Pending |
| Contract 1.5 / production-book-2 / resource-set 3 and capabilities | Pending; publish atomically only after gates |
| VMB exporter geometry / metrics / semantics / source mapping | Pending; companion design lives in VMB docs |
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
