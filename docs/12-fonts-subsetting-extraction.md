# Font、subset、text extraction

OpenType/TrueType初期profileではoriginal GIDとsubset GIDを別型にする。PDF resource blueprintをCIDFontType2 + FontFile2へ固定するため、admissionはstandalone/TTC各faceのsfnt scaler signatureがTrueType outline `0x00010000`であることを要求し、`OTTO`/CFFをfail closedで拒否する。`head`/`maxp`は重複なくbounds内に存在し、unitsPerEmはOpenType有効範囲16..=16384、glyph countはpositiveでなければならない。used glyphからcomposite component closureを固定点まで計算し、component GIDをsubset mappingへ書き換える。`.notdef`と必要metrics/table checksumを保持する。

late resource finalizerは次のmapping/policyを一つのPDF-profile-readyかつbackend identity-freeな`FrozenPdfResourcePlans`へ固定する。このphase以降はPDF-specificである。

- original GID -> subset GID
- CID -> subset GID (`CIDToGIDMap`)
- CID widths (`/W` and `/DW`)
- CID -> Unicode UTF-16BE (`ToUnicode`)
- cluster -> extraction policy

各font planはType0、CIDFontType2、FontDescriptor、embedded subset program、ToUnicode、CIDToGIDMapの6 indirect-object roleをexact blueprintとして持つ。plan中の全`OriginalGlyphId`は、同じledger/FontFaceIdへbindされたactual admitted `glyph_count`未満でなければならず、範囲外GIDをsubset/CID planへ流さない。descriptor metricsはnondegenerate FontBBox、positive StemV、required CapHeight、Symbolic/Nonsymbolic flagsのexactly-oneを満たす。deterministic subsetterはembedded programの`name` tableにあるPostScript名をdense FontInstanceIdから導出したunique six-uppercase-letter tag + `+Typaxis`へ書き換える。Profile 1.0 subset outputはformat 0 name tableにexactly one Windows Unicode BMP/English-US PostScript-name record `(platform=3, encoding=1, language=0x0409, name_id=6)`を持つ。sealed encoder owner自身がbounded table-directory/name-record parseでsubset bytesからこの値を再抽出し、caller supplied文字列を受けずreceiptへbindする。resource finalizerは期待名とのexact一致を再検証し、Type0/CIDFont/FontDescriptorはreceiptと同じ値をBaseFont/FontNameに使う。

six-letter uppercase tag namespaceはexactly `26^6 = 308,915,776`件である。`ResourceLimits.max_fonts`はこの値以下をProfile 1.0 maximumとし、exact maxは設定可能、max+1はconfig validationで拒否する。したがってcanonical dense FontInstanceIdからのbase-26割当は全許可configに対してtotalかつcollision-freeである。

CID 0は`.notdef`専用に予約し、各font instanceのCIDは1..`ResourceLimits.max_cids_per_font`だけを使用する。このlimitは65535以下でなければならない。distinct CID countを割当前にchecked計数し、limitを超えるplanはR7xxx `cid_space_exhausted` errorとしてfinalized planを返さない。同じsubset GIDへ複数CIDを対応させる場合もこの上限を共有する。

clusterのglyphごとのToUnicodeを連結して元Unicode列を正確に再現できない場合、clusterを一つのActualText marked-content spanで囲む。ActualTextはbackendだけが生成し、重複・nestを避ける。variation selector、combining sequence、non-BMP surrogate pairを保持する。

extraction planが参照するtextは`DisplayTextMap`でchecked remapされ、`DisplayDocument`のcanonical text-buffer tableが所有する`DisplayTextSpan`である。backendはinternal parsed/generated identifierを再解釈せず、selected stateからstable remap済みのDisplay bytesだけをToUnicode/ActualText生成へ使う。

Profile 1.0のDisplay FontInstanceIdは使用faceごとに1件で、binderがDisplayのFontFaceIdをDocument declarationとmanifest admitted recordへ照合して`(FontFaceId, admitted SHA-256 raw bytes)` keyを導出する。Display wire自体はhashを持たず、featuresはrun request-local、sizeはDrawGlyphRun-local、variationは未対応である。
