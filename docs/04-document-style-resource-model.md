# Document、Style、Resource model

Semantic ASTはparagraph、heading、list、table、figure、page breakと、text、emphasis、strong、link、anchor、reference、footnote reference、line breakを表す。text contentは`TextSpan`で参照し、glyphや座標を持たない。

footnote definitionはDocument直下の別collectionに置き、reference targetを意味検証する。NodeId、footnote ID、anchor IDはdocument内で一意。

`NodeId`はDocument rootを0とし、generic JSON member orderではなく型が宣言するchild-field順のpreorderで0-based denseに割り当てる。document blocksはarray順、block/inline/list/table/figure childは各型の宣言済みfield順で辿り、最後にFootnoteId UTF-8 byte順のdefinition nodeとそのchildrenを辿る。wire `node_id`はこのpreorder indexと一致する。footnote definitionsはFootnoteId UTF-8 byte順、page-master definitionsはMasterId UTF-8 byte順にするが、document blocks、style rules、selection rules、table cellsなど意味順を持つarrayはsortしない。

canonical listは`ordered`と`start`の同義別表現を持たない。`ordered = true`ではresolved positive integer `start`が必須で、sourceで省略された既定値もcanonical packageでは明示的な`1`にする。`ordered = false`では`start`を必ず`null`にし、integerを拒否する。Profile 1.0のListは1件以上のListItemを必須とし、空Listを許さない。ordered markerの最終値`start + item_index`はchecked u32加算で表現可能でなければならない。Tableは1列以上に加えて`head + body`に1行以上を必須とし、空Tableを許さない。これにより非空DocumentがFlow上だけblank扱いとなり、trusted Displayを発行できない妥当packageを作らない。

profile 1.0のstyleable blockは`paragraph`、`heading`、`list`、`table`、`figure`、`page_break`で、全blockがcanonical `classes` arrayを持つ。class tokenは`[A-Za-z_][A-Za-z0-9_-]*`だけを許し、arrayはduplicateを持たずUTF-8 byte順にsortする。matching時はarrayをsetとして扱う。

selector grammarは`block_type(.class)*`だけで、`block_type`は上記6種のいずれかとする。selectorのclass componentもduplicateを持たずUTF-8 byte昇順のcanonical orderにする。空白、combinator、`#id`、attribute、pseudo、空class、duplicate class、非canonicalなclass順を拒否する。selectorのblock typeがtarget kindと等しく、selector内の全classがtarget class setに含まれる場合だけmatchする。specificityは常に`(0, class_count, 1)`。

Style declarationは順序付きで、valueがtagged型を持つ。Profile 1.0のproperty registryは次の閉じた集合であり、unknown nameとname/value kindの不一致をS3xxx errorにする。同じrule内で同じpropertyを繰り返すことはでき、その場合も下記の`important`とorigin declaration orderを含む通常のcascadeだけでwinnerを決める。後続phaseはraw `StyleValue`をnameで再解釈しない。

| property | canonical value | 制約 | computed semantics |
|---|---|---|---|
| `font_family` | `font_family_list` | 1件以上の非空・非blank・control不在・重複なしfamily alias | 宣言順にResourceCatalogを検索して最初の既知aliasを選ぶ |
| `font_size` | `length` | 0より大きい | shapingとline metricsのem size |
| `line_height` | `length` | 0より大きい | blockのline advance |
| `page` | `keyword("auto")` または `string(PageName)` | 下記PageName規則 | 次page境界のnamed-page request |

`page`のinitial valueは`auto`である。ambient system fontやlocale依存defaultを使わないため、textまたはgenerated markerをmaterializeするblockはshaping開始前に`font_family`、`font_size`、`line_height`のwinnerをすべて持たなければならない。`ResolvedTextStyle` constructorが3値をtyped valueへ変換し、family listのどの名前も`AdmittedResources`のfont-family tableへ解決しない場合はF4xxx errorにする。このtableは全declared fontのadmission、hash、face index、metadata検証が完了した`AdmittedResources`だけが所有し、ParsedPackageの宣言だけからshaping対象を作らない。familyはFontFaceDeclarationが与えるlogical aliasであり、Unicode scalar sequenceをbyte-exact比較する。font内name table、case folding、Unicode normalization、system font searchから推測しない。ResourceCatalogのfamily aliasはProfile 1.0では一意であり、異なるFontFaceIdが同じaliasを宣言するpackageをS3xxx errorとして拒否する。これによりcollection orderをfallbackに使わない。

shaping用text receiptはtext/site ownerとstyle ownerを別々に固定する。parsed textではtyped AST上の唯一の`Inline::Text`がtext ownerで、そのnearest enclosing styleable blockがstyle ownerである。generated siteもregistryのownerを保持しつつnearest enclosing styleable blockをstyle ownerにする。例外としてDocument直下の`FootnoteDefinition` markerは、そのdefinition内をtyped preorderで走査して最初に現れるtext-producing `Paragraph`または`Heading`をcanonical style ownerにする。該当blockがなければfail closedとし、callerが別blockを選べない。

style IDは一意、`extends`は既知style IDまたはnullで、inheritance graphはDAGとする。style ruleのwire `source_order`はrules arrayの0-based indexと一致するdense canonical orderで、duplicate/gap/mismatchをS3xxx fatalにする。`declaration_order`はwire fieldではなく、origin ruleのdeclarations array indexからresolverが導出する。したがってdeclaration order自体にduplicate/gap/mismatch表現は存在しない。unknown parentとcycleはS3xxx fatalにする。

matchしたrule Rごとにextends chainをrootからRへ展開し、rootをinheritance depth 0、childほど大きいdepthとする。chain上の各origin declarationをRのmatch contextでcascadeし、property winnerは`(important, R_specificity, R_source_order, inheritance_depth, origin_declaration_order)`の辞書式最大で決める。`true > false`かつ各integerは大きい方が勝つ。完全tieは同じorigin declarationだけを表し、container iteration orderをtie-breakに使わない。

computed style property `page`はkeyword `auto`またはPageName lexical stringだけを受ける。`auto`は`requested_named_page = None`、stringは検証済み`Some(PageName)`へ変換する。その他のvalue kind、unknown keyword、invalid PageNameはS3xxx errorにする。flowのblock境界でeffective page nameが直前の値から変化した場合は、そのblockの直前でpage breakし、新しい値を次pageのselection inputにする。

style phaseはcomputed `PageName`だけを所有する。page開始時にPaginatorがflow position、page index、computed `page` propertyから`PageSelectionContext { page_index, requested_named_page }`を作り、physical page number、first、parityを`page_index`からcheckedに導出する。`PageName`は`[A-Za-z_][A-Za-z0-9_-]*`を満たす専用semantic typeであり、同じ文字列でも`StyleId`や`MasterId`へ暗黙変換しない。同一pageのmasterを途中で変更しない。

PageMasterSelectorは`PageSelectionContext`に対してruleをmatchし、ruleのnon-null `named_page`も同じ`PageName`型を使う。ruleの`named_page = null`、`first = null`、`parity = any`はそれぞれwildcardで、それ以外はcontextとのexact matchを要求する。`selector_specificity = (named_page指定, first指定, parityがany以外)`を0/1 tupleとし、`(selector_specificity, source_order)`の辞書式最大をwinnerにする。matchがなければdefault masterを使い、winnerを付与した`PageContext { selection, master_id }`を構築してFragmenterへ渡す。`source_order`はselection_rules arrayの0-based indexと一致するdense canonical orderで、duplicate/gap/mismatchと異なるruleの完全tieをS3xxx fatalにする。masters collection内の定義`master_id`だけを一意にし、default/ruleの参照は既知masterを指せば複数rule間で重複してよい。page width/heightは正で、body/header/footer/footnote Rectはpage内に収まらなければならない。

ResourceCatalogは一意なlogical font face/image IDの宣言を持つ。`AdmittedResourceResolver`はParser後・shaping前に許可root内のbytesを一度だけopenしたopaque source receiptを受け、per-resource/count/aggregate limitを超えるreadを開始する前に拒否してから、hash、face index、shapingに必要なfont metadata、image dimensionsをimmutable `AdmittedResourceLedger`へ固定する。caller-supplied lengthや任意readerはtrusted admission inputにならない。layoutとDisplay Listはlogical IDを使うが、shaping cacheはadmitted font hashを参照する。Display ListまではPDF非依存である。late resource finalizationはprofile 1.0のPDF-specific phaseとしてsubset/CID/extraction/descriptor metrics/image encodingを`FrozenPdfResourcePlans`へ固定する。このplanはPDF-readyだがbackend identity-freeであり、backend handle、PDF name、object IDを所有しない。

`FontFaceId`と`ImageResourceId`はexpanded declaration orderでそれぞれ0からdenseに割り当て、resource declaration array indexと一致させる。include/resource workerの完了順でIDを決めない。
