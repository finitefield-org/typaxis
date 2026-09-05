# VMB book production implementation evidence

Status: In progress. This ledger does not narrow the scope of
[design 28](28-vmb-book-production-compatibility.md). No full-book success or
Harano support is claimed until the corresponding gates have evidence.

| Requirement | Current evidence / remaining work |
| --- | --- |
| ADR-0038 lexical exception | Decision recorded; parser changes under test |
| Safe-SVG 2 whitespace, multiple paths, curves, subpaths | Implemented; initial 13 focused tests passed; added detailed-error / real-SVG tests still running |
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
| TrueType full book, one package / PDF | Pending |
| Unchanged Harano full book, one package / PDF | Pending |
| Independent visual / baseline / spacing / extraction / tag verification | Pending |
| Performance, determinism, negative/tamper tests, old-profile regression | Pending |
| macOS / explicitly managed Linux host evidence | Pending |

## Execution record

Implementation branch: `codex/vmb-book-production`. The preceding goal turn only
restated the design and was no progress. The current turn modifies authoritative
product code and adds regression tests; test results will be recorded when complete.


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
- `implementation-chapter-04`: uses one shared TOML with contract 1.4 and `pdf_stream_compression = "none"` for both commands; execution pending at this ledger update.
- `implementation-book-font-probe-01/observed-02.json`: diagnostic-only full-book copy with the font replaced by the chapter TrueType passes the image count budget but fails at `P1102: semantic source span ownership mismatch`. This adds an exporter/source-mapping investigation; no source-span validation was relaxed. The original snapshot was preserved. An initial invocation had duplicated a configured root and was corrected by using a separate working directory.

Known remaining SVG-detail work includes precise context for every transform/paint/non-path geometry failure, exact segment context on budget failures, context for a zero remaining document budget and clip-reference replay positions. The first detailed-error tests do not close those requirements. Complete positive resource-count boundaries, 5,000 placed distinct resources and performance observations also remain pending.
