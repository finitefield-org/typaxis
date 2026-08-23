# Cross-layer contract matrix

| Contract | Rust | JSON | Docs | Validator |
|---|---|---|---|---|
| product/CLI identity | `typaxis_core::PRODUCT_NAME` / Cargo `[[bin]]` | manifest `engine.name` | docs/19 | exact name/bin/Schema checks |
| wire ID | `typaxis_core::CONTRACT` | root `contract` | contract-version | exact scan |
| source/text/local map range | `SourceSpan` / `TextSpan` / `Utf8ByteRange` | common + document package | docs/03 | bounds/boundary/coverage |
| path containment | `PortablePath` | portable path def | docs/18 | lexical + canonical containment |
| length and transform | `Length` / `AffineTransform` | common defs | docs/24 | numeric/type checks |
| parser package | `ParsedPackage` | document root | docs/03,04 | Rust token + Schema |
| bidi and shaping | `BidiLevel` / `ShapeRequest` | display run | docs/05 | level/range/order |
| paragraph items | `ParagraphItem` / `BreakKind` | internal IR | docs/06,07 | Rust contract token |
| reflow | `Fragmenter` / `Continuation` | trace fragments | docs/08 | progress invariant |
| convergence | state-indexed passes | layout trace | docs/09 | chain/stable/cycle/selection |
| Display ops | `DISPLAY_COMMAND_OPS` | exact enum | docs/11 | exact set comparison |
| text paint/destinations | `Paint` / `NamedDestination` | display root/run | docs/11,15 | resolution and bounds |
| path/stroke | `Path` / `DashPattern` | command schema | docs/11 | arity/state/dash |
| cluster extraction | `DisplayCluster` | cluster objects | docs/12 | span/overlap/order |
| subset plan | `FontSubsetPlan` | finalized internal | docs/12 | Rust plan tests |
| resource finalization | typed font/image plans | build records | docs/13 | manifest/file facts |
| PDF stream | `PdfStreamObject` | N/A | docs/14 | reserved-key source invariant |
| frozen page tree | builder/frozen types | N/A | docs/14 | Rust token/tests |
| limits | `ResourceLimits` | config Schema | docs/18 | exact field set + relations |
| build manifest | `typaxis-manifest` | build Schema | docs/16 | unique/order/hash/bytes |
| diagnostics | `DiagnosticCode` | diagnostic pattern | docs/17 | exact category set |
| archive | release builder | MANIFEST | docs/16 | metadata/order/safety/rebuild |

Contract変更時はRust、Schema、positive/negative fixture、docs、validatorを同じchange setで更新する。
