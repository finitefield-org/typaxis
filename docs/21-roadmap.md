# Implementation roadmap

このroadmapはtargetの実装順であり、各milestoneの完了記録ではない。契約・Schema・reference type/testが存在しても、公開CLIからPDFまでend-to-endで到達できるとは限らない。現行reference workspaceの到達範囲と、machine inputを含む未実装項目は[docs/25](25-machine-input-pdf-improvements.md)を正とする。特に、M1に記載したJPEGは現行runtimeで未実装であり、画像admissionはPNGだけ、figure paint経路も未完成である。

M0 contract: scoped core newtypes、separate SourceCatalog/TextStore ownership、`ValidatedParsedPackage`/`ParseOutcome`/`AdvisoryDiagnostic`、SafeUri、structured Fragmenter continuation、Display/PDF model、validator。

M1 minimal PDF: path/JPEG、Type0/CIDFontType2、CIDToGIDMap、ToUnicode、absolute Japanese run、xref。

M2 text: admitted resource resolution、grapheme/bidi/itemization/fallback/shaping、line-level UAX #9 reorder、cluster extraction round-trip。

M3 paragraph: UAX #14、Japanese pair table、greedy/optimal、justification。

M4 flow/pagination: LayoutPassCoordinator feedback、paragraph/heading/list/image、page masters、keep/widow/orphan、scored fallback、trace convergence。

M5 table/footnote/reference: basic table、bounded footnote、TOC/page reference。

M6 hardening: deterministic spool/release、limits、fuzzing、renderer/extractor matrix、accessibility investigation。

各milestoneは受入testで完了判定する。
