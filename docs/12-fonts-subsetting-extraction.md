# Font、subset、text extraction

OpenType/TrueType初期profileではoriginal GIDとsubset GIDを別型にする。used glyphからcomposite component closureを固定点まで計算し、component GIDをsubset mappingへ書き換える。`.notdef`と必要metrics/table checksumを保持する。

resource finalizerは次を同じplanから生成する。

- original GID -> subset GID
- CID -> subset GID (`CIDToGIDMap`)
- CID widths (`/W` and `/DW`)
- CID -> Unicode UTF-16BE (`ToUnicode`)
- cluster -> extraction policy

clusterのglyphごとのToUnicodeを連結して元Unicode列を正確に再現できない場合、clusterを一つのActualText marked-content spanで囲む。ActualTextはbackendだけが生成し、重複・nestを避ける。variation selector、combining sequence、non-BMP surrogate pairを保持する。
