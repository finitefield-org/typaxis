# Testing strategy

unit: fixed-point conversion/rounding、UTF-8 boundary、text map、style precedence、line break tie、cursor epoch、transform composition、PDF name/string、duplicate object、xref offset。

conformance/differential: Unicode grapheme/linebreak/bidi data、reference shaper、subset reparse、multi-renderer raster、multi-extractor Unicode。

property/fuzz: arbitrary source/font/imageでpanicなし、fragment progress、bounded pass、balanced graphics state、all PDF references resolved、composite closure、archive path safety。

golden: canonical AST、trace、Display List、resource/build manifest、PDF bytes。意図変更では理由をreviewしcontract versionを判定する。
