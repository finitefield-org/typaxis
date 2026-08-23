# 全体アーキテクチャ

```text
SourceFile(s)
 -> Parser / IncludeResolver
 -> ParsedPackage { sources, text_store, document, styles, page_masters, resources }
 -> StyleResolver
 -> TextPipeline
 -> ParagraphBreaker
 -> FlowTree
 -> Fragmenter(fragment request, cursor)
 -> Paginator(pass chain)
 -> PageFragmentTree
 -> DisplayDocument
 -> ResourceCollector / Finalizer
 -> FinalizedResources
 -> PdfObjectGraphBuilder
 -> FrozenPdfObjectGraph
 -> PdfSerializer
```

各phaseはimmutable inputとdiagnosticsを返す。現在時刻、system locale、system font search、random、HashMap iteration order、thread completion orderを暗黙入力にしない。
