# Re-entrant fragmentation

pagination前に固定heightのfragment列を作らない。`Fragmenter`は毎回、frame、page context、reserved footnote height、FlowCursor、LayoutEpochを受ける。

結果はplaced fragments、next cursor、discovered footnotes、anchors、progress keyを返す。cursorはowner、epoch、canonical boundary keyを持ち、別document/style/font generationへ流用できない。

同じrequestから同じresultを返す決定性が必要。next cursorが同じ、fragmentが空、occupied block sizeが0の組合せはzero progressとして停止する。
