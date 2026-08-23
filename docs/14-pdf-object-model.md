# PDF object model

初期profileは新規PDF 1.7互換生成、classic xref table、generation 0。`PdfValue`はdirect valueだけでstreamを持たない。streamは`IndirectObjectBody::Stream`のみ。

`PdfObjectGraphBuilder::insert`はduplicate時に最初のobjectを保持する。`freeze`は全reference、root、page tree、stream dictionary予約keyを検査し、以後mutation不能なgraphを返す。Catalogの`/Pages`は必ずroot `/Pages` dictionaryを指し、そのroot nodeは`/Parent`を持たない。

`/Length`は圧縮後data bytesからserializerが生成し、caller dictionaryに指定させない。PdfNameはraw bytesを保持し、whitespace、delimiter、`#`、非regular byteを`#XX` escapeする。ユーザー文字列をraw tokenとしてwriteしない。

classic xref offsetは10 decimal digitsのため、outputが10,000,000,000 bytesへ達する前に失敗する。
