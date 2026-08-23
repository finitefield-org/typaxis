# PDF text、link、coordinate mapping

内部座標は左上origin・Y下向きで、column vectorに`x' = a*x + c*y + e`、`y' = b*x + d*y + f`を適用する。`concat_transform(M)`は`CTM := CTM * M`。高さ`H`のpage content開始時にroot matrix `(a=1,b=0,c=0,d=-1,e=0,f=H)`を一度だけ適用し、二重反転を禁止する。matrix compositionはi128 intermediate、round-half-to-even、overflow check。

Type 0 fontはIdentity-H、CIDFontType2、FontDescriptor、embedded TrueType font program、CIDSystemInfo、CIDToGIDMap、W/DW、ToUnicodeの6 indirect-object closureを一つのfrozen planから整合させる。admissionでTrueType outline `0x00010000`とunitsPerEm 16..=16384を証明し、`OTTO`/CFFをこのFontFile2 pathへ流さない。deterministic subsetterはembedded programのPostScript `name`をdense FontInstanceIdをbase-26化したsix-uppercase-letter subset tagと`+Typaxis`へ書き換え、sealed receiptでその抽出値を証明する。Type0/CIDFont/FontDescriptorのBaseFont/FontNameはそのreceipt値をexactに共有する。FontDescriptorはFontName、Flags、FontBBox、ItalicAngle、Ascent、Descent、CapHeight、StemV、FontFile2を必須とし、bboxはnondegenerate、StemVはpositive、Symbolic/Nonsymbolic flagsはexactly oneにする。CID stringはbig-endian 2 bytes。

annotationとdestinationはcontent stream CTM外にあるため、PDF backendが別に変換する。named destinationのpage pointはselected pagination receiptのframe-local anchor pointとexact selected frame originからDisplay builderが導出し、PDF callerが再構築しない。内部point `(x,y)`はPDF point `(x,H-y)`へ変換する。annotation Rectは4 cornerを変換後、`[min_x,min_y,max_x,max_y]`へ正規化する。XYZ destination pointとFitWidthのnon-null topも同じY変換を使い、null topとFitPageは座標を持たない。

URI linkはsyntax境界でallowlisted scheme、control/whitespace/NUL、raw UTF-8 lengthを検証済みの`SafeUri`だけを受ける。internal linkはCatalogのcanonical `/Names << /Dests ... >>` entryへ解決し、各Display annotationはpage encounter orderで1 indirect `/Annot /Subtype /Link` objectになり、owning pageの`/Annots`からexactly once参照される。JavaScript、Launch、embedded-file action、raw action dictionaryは初期profileで生成しない。
