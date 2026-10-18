# Actual-engine thin-stroke fixtures

Eight authored cases cover a fraction rule and a separate diagonal negation overlay in inline/block form at 11pt and 786431/65536pt. The TeX and semantic alternatives are authored test inputs. These are not extracted book occurrences or approved layout goldens.

The source engine corpus is retained unchanged under `engine/`. `cases.json` binds source/admitted SVGs and physical metrics; `occurrences.json` fixes the eight source owners and their order. Path 2 is the fraction rule or the independent U+0338 diagonal. The grouped equality in `a\not{=}b` avoids the engine's precomposed not-equal replacement.

`provenance.json` records VMB commit 9efff4a plus the explicit Go overlay, producer source and binary hashes, and the existing license/notice records. `producer-original.go` and `producer-overlay.go` are reproducible input records, not files to build together in this directory. Apply the overlay to the existing `tools/typaxis-math-fixtures/main.go` at that VMB revision. The executable and build/generation evidence are retained separately in checkpoint 305.

Formula rendering and thin-stroke damage checks do not establish body-glyph visibility, mixed text spacing, independent layout, original-book acceptance, or human PDF/UA acceptance.
