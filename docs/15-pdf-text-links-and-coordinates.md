# PDF text、link、coordinate mapping

内部座標は左上origin・Y下向き。page content開始時にPDF user spaceへroot transformを一度だけ適用し、二重反転を禁止する。matrix compositionはi128 intermediate、round-half-to-even、overflow check。

Type 0 fontはIdentity-H、CIDFontType2、FontDescriptor、CIDSystemInfo、CIDToGIDMap、W/DW、ToUnicodeを整合させる。CID stringはbig-endian 2 bytes。

URI linkはallowlisted schemeのみ。internal linkはnamed destination/explicit destinationへ解決する。JavaScript、Launch、embedded-file action、raw action dictionaryは初期profileで生成しない。
