# Rust workspace境界

`typaxis-core`を最下層とし、document/styleは互いに依存しない。syntaxだけがsources/text/document/style/resourcesを束ねる。layoutはPDFを知らず、PDFはDocument ASTを知らない。

禁止edge:

- `typaxis-core -> workspace crate`
- `typaxis-document <-> typaxis-style`
- `typaxis-layout -> typaxis-display-list|typaxis-pdf`
- `typaxis-pagination -> typaxis-pdf`
- `typaxis-pdf -> typaxis-document|typaxis-style|typaxis-layout|typaxis-pagination`

公開ID、offset、length、fingerprintはnewtypeとする。source/text offsetに裸の`usize`を使用しない。
