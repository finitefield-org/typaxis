# Source、TextStore、Parser契約

`SourceCatalog`はUTF-8 sourceのURI、byte length、SHA-256を持つ。`TextStore`はparser/normalizerが後段へ渡す正確なUnicode textを保持する。

`SourceSpan`は原source、`TextSpan`はTextBufferを指す。CRLF正規化、escape展開、generated textは`TextMapSegment`で対応付ける。

```text
identity    text bytesとsource bytesが1:1
replacement 正規化・escape展開など。長さ一致を要求しない
inserted    source spanを持たないgenerated text
```

segmentは非空で、buffer全体を隙間なく覆い、順序・重複・UTF-8境界を検証する。空segmentは同じmappingの非canonicalな別表現になるため拒否する。Unicode正規化は既定で行わない。NFC等を行う場合は明示設定とreplacement mapが必要。

Parserはfatal時にpackageを返さない。回復ASTを返す場合もunknown IDや壊れたtext mapを後段へ流さない。includeは許可root内、canonical path cycle、深さ、file数、総bytesで制限する。
