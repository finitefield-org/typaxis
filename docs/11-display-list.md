# Display List契約

Display ListはPDF非依存。logical font/image ID、typed paint、dimensioned transform、cluster-level text mappingを持つ。PDF name、CID、object number、raw action dictionaryは禁止。URI link targetはsyntax境界で検証済みの`SafeUri`だけを受け取る。

`DisplayDocument.pages`はarray indexと一致する0-based dense `page_index`を持ち、named destinationの`page_index`も同じidentifier spaceを参照する。CLIのphysical page numberとは別である。

Display rootは`source_layout { state_fingerprint, layout_epoch }`を必須とし、選択されたmaterialized trace stateのfingerprintとepochにexact一致させる。`DisplayDocument::from_untrusted_parts_for_selected_pagination`はwire/fixtureの構造検査用であり、selected stateを付与してもpaint provenance receiptにはならない。`StructurallyValidatedDisplayDocument`は`ValidatedParsedPackage + selected PaginationResult + EffectiveConfig`に対してparsed/generated text、page count/index、master、width/height、command/annotation構造を再照合するが、publication用tokenへ変換できない。

publication用`ValidatedDisplayDocument`はprivate field constructorを持ち、crate-owned `DisplayListBuilderOwner`だけがpackage/pagination-bound `DisplayTextMap`と実layout-to-paint出力を消費して発行する。owner capabilityを外部callerは構築できず、任意commands/destinations/pagesをselected fingerprintでstampしてtrusted Displayにしない。named destinationsはselected `PaginationResult`の`PlacedAnchor`完全集合からだけ導出し、frame-local pointへ選択page frame originを加えたpage pointを`Xyz` viewとして保持する。AnchorId UTF-8 byte順で、missing/extra/duplicate ID、wrong page/frame/column/pointを拒否する。`paint_reference_paragraphs`はselected passが所有するexact FlowTree paragraph registryとfragment rangesからcluster-scoped glyph commands、bidi level、parsed/generated Display spanを導出する。`paint_reference_selected`はblank documentまたはdirect anchorだけのempty paragraph、`paint_blank_selected`は完全blank domainを扱う。いずれもselected geometry/anchor closureを完全導出し、caller paint payloadを受けない。resource finalizerとPDF backendはこれらpaint-owner-issued typeだけを受ける。source layout receiptはPDF objectやsubset hashを持たない。

`DisplayDocument.pages`は常に1件以上を持つ。empty documentはdocs/09でmaterializeしたdefault-master blank pageを、空command/annotation collectionを持つpage index 0として表す。

command:

- save / restore
- concat_transform
- clip_path
- fill_path
- stroke_path
- draw_glyph_run
- draw_image

linkは描画commandではなくpage annotation collection。annotation rectangleとnamed destinationは内部page座標で保持し、content stream CTMの対象にしない。PDF backendはpage heightを用いて別途PDF user spaceへ変換する。path verbはmove/line=1 point、curve=3 points、close=0 point。paint/clip対象pathは`move_to`で始まり、少なくとも1本のlineまたはcurveを持つ。save/restoreはpageごとにbalanceする。

Display List wire上のtext referenceは0-based dense `DisplayTextBufferId`を持つ`DisplayTextSpan`だけで、internal `TextSpan`/`GeneratedTextSpan`を混在させない。`DisplayTextMap::from_selected_spans`は`ValidatedParsedPackage`が所有するparsed `TextStore`とselected passが所有する`GeneratedTextStore`だけからimmutable mapを作り、package document/style epochとselected layout epochを照合する。trusted Display constructionはこのmapを値として消費し、全GlyphRun/cluster spanはそのbufferだけを参照する。clusterがUnicode所有単位であり、cluster内の複数glyphへ同じUnicode列を複製しない。

各DisplayTextBufferは`origin = parsed{text_buffer_id}`または`generated{key}`を持つ。参照されたparsed bufferをTextBufferId順、その後にgenerated bufferをGeneratedBufferKey順で配置し、wire `text_id`を0-based denseにする。bytesは元bufferとexact一致し、未使用buffer、unknown origin、duplicate originを拒否する。

root `font_instances`はPDF-independentな`{font_instance_id,font_face_id}` mappingだけを持ち、admitted hashを重複保存しない。使用font faceをmanifest admitted recordへbindして得るcanonical key `(FontFaceId numeric, admitted_sha256 raw)`順に0-based dense FontInstanceIdを割り当て、同じfaceのduplicateとsparse/noncanonical IDを拒否する。全page/command encounter orderのDrawGlyphRun `run_id`も0-based denseにする。

DrawGlyphRunはvisual orderの`glyphs` arrayとlogical orderの`clusters` arrayを分離する。clusterはdense `logical_ordinal`、非空`[glyph_start,glyph_end)`、extractionを持ち、glyph rangesはcluster array順に増加する必要はないが、互いにdisjointで全visual glyph indexをexact coverする。Unicode clusterのTextSpanはlogical ordinal順にrun TextSpanを隙間なくcoverする。これによりRTL visual orderをlogical text ownershipへ偽装しない。
