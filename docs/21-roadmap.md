# Implementation roadmap

M0 contract: scoped core newtypes、separate SourceCatalog/TextStore ownership、`ValidatedParsedPackage`/`ParseOutcome`/`AdvisoryDiagnostic`、SafeUri、structured Fragmenter continuation、Display/PDF model、validator。

M1 minimal PDF: path/JPEG、Type0/CIDFontType2、CIDToGIDMap、ToUnicode、absolute Japanese run、xref。

M2 text: admitted resource resolution、grapheme/bidi/itemization/fallback/shaping、line-level UAX #9 reorder、cluster extraction round-trip。

M3 paragraph: UAX #14、Japanese pair table、greedy/optimal、justification。

M4 flow/pagination: LayoutPassCoordinator feedback、paragraph/heading/list/image、page masters、keep/widow/orphan、scored fallback、trace convergence。

M5 table/footnote/reference: basic table、bounded footnote、TOC/page reference。

M6 hardening: deterministic spool/release、limits、fuzzing、renderer/extractor matrix、accessibility investigation。

各milestoneは受入testで完了判定する。
