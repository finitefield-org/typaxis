# Text pipeline

```text
TextSpan
 -> grapheme boundaries
 -> bidi paragraph/runs
 -> script/language itemization
 -> cluster-safe font fallback
 -> shaping into glyphs + cluster groups
 -> Unicode line-break candidates
 -> Japanese profile adjustment
 -> break selection
 -> UAX #9 line-level L1 reset
 -> final-line reshape
 -> justification
 -> UAX #9 L2 visual reorder
```

paragraph bidi resolutionで得たembedding levelはimmutable shaped run sliceへ保持する。break確定後、各lineでUAX #9 L1のline-level reset、final reshape、justification、L2 visual reorderの順に実行する。その後のGlyphRunはvisual glyph orderとlogical TextSpanを所有するcluster groupを分離する。RTLでcluster順がvisualに並んでもsource/text mappingを失わない。

break位置はparsed `TextBuffer`または`GeneratedTextBuffer`のUTF-8境界であり、shaping cluster内部へ置かない。`LayoutPassCoordinator`は各materialized stateについてimmutable `GeneratedTextStore` overlayを所有する。overlayはparsed `TextStore`と別の`GeneratedTextBufferId` identifier spaceを使い、その範囲は`GeneratedTextSpan`で表す。state-dependent reference/counter/marker textはpassごとにworking overlayへ固定し、通常textと同じbidi、shaping、line-break処理へ入力する。state nのpagesは同stateが所有する`G_n`でpaintされた事実を保持し、次pass用`G_{n+1}`をstate nのfingerprintへ先取りして混ぜない。parsed `TextBufferId`とgenerated IDの数値が同じでも同一bufferとして扱わない。

profile 1.0はallocation IDを含まない`GeneratedBufferKey = (owner: NodeId, generation_kind, owner_local_ordinal)`を使い、同じ`LayoutEpoch`内でkeyを一意にする。`generation_kind`はclosed enumで、全memberとsort orderは`page_reference < counter < list_marker < footnote_marker < discretionary`とし、unknown valueを許さない。overlay builderは全key/bytesを収集後、keyを`NodeId` unsigned数値、上記kind順、ordinal unsigned数値の辞書式昇順へsortし、0からdense `GeneratedTextBufferId`を割り当てる。duplicate keyはbytesが同じでもerrorにし、insertion/thread completion orderを使わない。overlayのReferenceFingerprintは`{"algorithm":"typaxis.reference-state.jcs-sha256/1","resolved_generated_text":[...]}`のcanonical JCSをhashし、algorithm IDを必ずdomain separationへ含める。

generated site registryはDocument typed preorderから一義に導出する。`Reference(format=page)`はreference node ownerの`page_reference`、`Reference(format=text|number)`は同ownerの`counter`、ordered/unorderedを問わず全ListItemはitem ownerの`list_marker`、FootnoteDefinitionとFootnoteReferenceは各node ownerの`footnote_marker`、SoftBreakはそのnode ownerの`discretionary`を1 siteずつemitする。各`(owner, kind)`内のordinalはsite emission順に0からdenseにする。各state overlayはこのregistryの全siteをちょうど1 recordずつ持ち、未解決または空の結果も`utf8 = ""`として保持する。missing、extra、duplicate、owner/kind/ordinal mismatchを拒否し、siteが0件のDocumentだけが空storeを持てる。

Profile 1.0のlist marker bytesはpackage ownerがcanonical ASTからmaterializeする。orderedは`ASCII base-10(start + item_index) + "."`で、item_indexは0-based、加算はchecked u32としoverflowをpackage validationで拒否する。unorderedはUTF-8のU+2022 BULLET (`•`) だけである。markerと本文の間隔はline-layoutのGlueでありmarker bytesへ空白を含めない。`PackageGeneratedTextBinding`は全`list_marker` bufferをこの値とexact比較し、caller-authored marker bytesをshapingへ昇格させない。

`PackageShapeTextReceipt`はsite/text owner、canonical style owner、package document fingerprint、exact UTF-8 rangeを持ち、generatedの場合はさらにselected overlayのreference fingerprintと`GeneratedProvenance`を持つ。parsed rangeはtyped AST上の単一`Inline::Text`宣言span内、generated rangeは当該`PackageGeneratedTextBinding`のregistered site内でなければ発行しない。main/pre/post contextは同じstyle ownerに属し、全receiptが`ShapeFontSelectionReceipt`のstyle ownerと同じでなければならない。さらにcrate-owned canonical itemizerだけがlogical text stream上でmainに直結するpre/post rangeを選び、任意のsame-owner rangeをcontextとして選ばせない。generated receiptはexact `LayoutEpoch.references`とも一致させるため、別packageの同値TextBufferIdや別state overlayを混在できない。FootnoteDefinition markerのsite ownerとstyle ownerの規則はdocs/04に従う。

`ReferenceFingerprint`はGeneratedBufferKey順のrecordsを`{"algorithm":"typaxis.reference-state.jcs-sha256/1","resolved_generated_text":[{"key", "start_byte", "end_byte", "utf8"}, ...]}`というwrapper付きrecordにし、RFC 8785 JCS UTF-8 bytesのSHA-256を取る。array単体hash、algorithm無しrecord、別binary encoding、owner fieldの別名を使わない。page-reference provenanceはowner Referenceの`format = page`とtarget AnchorIdに一致しなければならない。

`GeneratedProvenance`は`GeneratedBufferKey`と、keyから導出済みのbufferを指す`GeneratedTextSpan`を持つ。logical identityとcanonical comparisonは`(GeneratedBufferKey, start_byte, end_byte)`であり、allocationされたbuffer ID自体を比較keyにしない。このtupleは同じ`LayoutEpoch`内で一意でなければならず、NodeIdだけをgenerated identityとして使わない。materialized stateはpagesを生成したoverlayを所有し、selected stateのoverlayはDisplay構築が完了するまで保持する。resolved textのUTF-8 bytes自体もpagination state fingerprintへ含める。initial `G_0`はvalidated package/limitsからpagination ownerが内部生成し、caller-supplied storeを受けない。次working overlayはprevious exact pages/placed anchors/package site registryへbindしたsealed `ReferenceTransitionReceipt`からだけ得る。reference workspaceはsite 0件のcanonical empty seedとunchanged transitionだけを発行し、nonempty siteはruntime resolver未実装としてfail closedにする。

Display構築時にinternal `TextBufferId`と`GeneratedTextBufferId`を同じwire `text_id`へ直接castしない。selected stateから参照されるparsed buffersを`TextBufferId`数値順、その後にgenerated buffersを`GeneratedBufferKey`順でstable配置する。同じbufferへの複数参照は1回だけ配置し、同じkeyが異なるbuffer/bytesを指す場合はerrorにする。`DisplayTextMap`が0-based dense `DisplayTextBufferId`へのchecked remapを所有し、trusted builderがremap済みbytesを`DisplayDocument.text_buffers`のcanonical tableへ移して全GlyphRun/cluster spanを`DisplayTextSpan`にする。このtableをDisplay artifact自身が所有するため、内部2 namespaceの数値衝突、生成時のinsertion order、state破棄でwire spanが変化・danglingしてはならない。

`ShapeTextView`はparsed `TextStore`または当該stateの`GeneratedTextStore`が発行したspan/viewだけを受け、callerの無関係なraw stringとprovenanceを組み合わせない。`ShapeRequest`にはpublic constructorを置かず、crate-owned canonical itemizerだけがpackage-issued viewsからmainと直結context、derived bidi/script、profile-owned language/featuresを組み立てる。Profile 1.0のDocument/Styleにはlanguageやexplicit OpenType feature入力がないため、canonical requestは`language = None`かつ`features = []`だけを許し、nonempty caller-local値はshaping work前に拒否する。itemizer owner constructorはmain text、pre-context、post-contextのUTF-8 byte数をchecked加算し、`max_shaping_context_bytes`以内であることをwork前に証明する。font inputは`typaxis-layout-contract`が発行した`ShapeFontSelectionReceipt`だけである。発行時に`ValidatedParsedPackage`、そのpackageの`PackageComputedStyle`、exact `AdmittedResourceLedger`、同ledger由来のcanonical dense `AdmittedFontInstanceTable`、`LayoutEpoch`を同時照合し、computed `font_face_id`と一致する唯一のinstanceを内部選択する。`ShapeRequest`は同じexact epochも再照合する。caller-supplied `FontInstanceId`、別ledgerの`AdmittedFontInstanceRef`、hash、face index、raw font bytesを個別入力として受けない。sizeはDisplay command-localであり、Profile 1.0のFontInstance identityへ任意に混ぜない。shaping cache keyはreceiptが保持する実際のadmitted font bytes hash、face index、text bytes、direction、script、canonical language/features、pre/post contextに加え、実行したShaperIdentityのbackend/versionとShapeRequestが保持するResolvedDataTablesのUnicode/Japanese line-break versionsを含む。allocation IDだけをfont identityとせず、別backend/versionまたは別registered data table setの結果をaliasしない。

Unicode property/Japanese line-break lookupはConfigLoaderがregistered selectorから解決したimmutable `ResolvedDataTables`だけを受ける。Profile 1.0の`ShaperIdentity { backend, version }`はpublic arbitrary string constructorを持たず、linked registry selectionのclosed identity factとしてdocs/16の値だけを表す。ただしidentity値だけは「そのimplementationが実際に全runをshapeした」ことを証明するcapabilityではない。reference workspaceのmanifestは登録済みbackend selectionを記録するだけで、完成runtimeがactual-useを主張する場合はsealed shaper-session receiptをshape outputからpagination/publicationまで伝播しなければならない。
