# 全体アーキテクチャ

```text
Host INPUT / project root / --config / repeatable --resource-root
 -> HostAdmissionContext (never serialized)

Host OUTPUT / trace / manifest paths / --force
 -> BuildExecutionContext (host paths never serialized)

Config defaults / typaxis.toml / TYPAXIS_ environment / CLI
 -> ConfigLoader / CLI
 -> EffectiveConfig

BuildExecutionContext + EffectiveConfig
 -> BuildOutputCommitContext (every build; non-cloneable one-shot sink owner)
 -> ManifestPublicationContext (only when manifest requested; same output session)

SourceFile(s) + HostAdmissionContext
 -> Parser / IncludeResolver (admitted-root relative)
 -> ParsedPackage { sources, text_store, document, styles, page_masters, resources }
 -> ParseOutcome::Parsed { package: Box<ValidatedParsedPackage>, advisory diagnostics }
 -> AdmittedRootSet (HostAdmissionContext-bound sealed capability)
 -> typaxis-resource-admission::AdmittedResourceResolver
 -> AdmittedResourceLedger { immutable bytes, hashes, face/image metadata }
 -> StyleResolver(AdmittedResourceLedger family table -> ResolvedTextStyle)
 -> typaxis-layout-contract::ShapeFontSelectionReceipt
      (validated package + computed style + exact ledger + canonical instance table + LayoutEpoch)
 -> LayoutPassCoordinator(state 0, limits)
     pass 0 input: canonical package-derived zero-site seed overlay G0
     pass i>0 input: previous materialized fingerprint F_i
       + sealed ReferenceTransitionReceipt(P_i -> working overlay G_{i+1})
      -> immutable GeneratedTextStore working overlay G_{i+1}
       -> TextPipeline(sealed font-selection receipt; no caller-selected font ID/hash/bytes)
       -> ParagraphBreaker
       -> FlowTree
       -> Paginator(page plan only; never shapes text)
          -> PageSelectionContext(page index + computed PageName)
          -> PageMasterSelector -> PageContext / frame request
          -> Fragmenter(frame request, continuation)
     pass 0 output: materialized state 1 owning P_1 + G_0
     pass i>0 output: materialized state i+1 owning P_{i+1} + G_{i+1}
      stable / cycle / max-pass -> selected materialized state
 -> PageFragmentTree(selected state)
 -> DisplayDocument (PDF-independent)
 -> ResourceCollector / LateResourceFinalizer
 -> FrozenPdfResourcePlans { subset, CID, extraction, descriptor metrics, image encoding }
 -> PDF backend (low-level caller graph builder remains untrusted)
      preflights every font/image/page/annotation object role
      assigns PDF resource names and object IDs
      materializes Names/Dests and page Annots closure
 -> FrozenPdfGraph
 -> PdfSerializer
 -> VerifiedPdfBytesReceipt
 -> BuildOutputCommitContext
      manifest omitted: commit PDF only
      manifest requested: consume sealed built preflight, commit PDF then canonical manifest
```

`LayoutPassCoordinator`だけがstate feedback loopを所有する。materialized state nはpages `P_n`と、それらを実際に生成したoverlay `G_n`だけを所有する。次pass前にsealed `ReferenceTransitionReceipt`がexact `P_n`、placed anchors、package generated-site registryをbindしてworking `G_{n+1}`を導出する。`LayoutPassInput`はstateそのものではなく「previous materialized fingerprint `F_n` + transition済みworking overlay `G_{n+1}`」であり、outputは`P_{n+1} + G_{n+1}`である。resolved reference text、frame width、footnote reservationなどstate依存入力が変わるたびにTextPipeline、line breaking、FlowTreeを再構築する。Paginatorはpage indexからselection contextを作り、PageMasterSelectorとFragmenterを駆動してmaterialized page planを返すが、text shapingを直接実行しない。

reference workspaceはtransition trust境界だけをcompile-checkする。`InitialPaginationState::new(&flow, &package, &limits)`はpackage generated-site registryが0件であることを確かめてcanonical empty `G_0`を内部生成し、siteが1件以上ならpass 0前に`UnsupportedReferenceTransition`でfail closedにする。site 0件の次passだけはexact unchanged overlay transitionをdeterministically発行できる。reference/counter/list/footnote/discretionary resolution runtimeは実装済みと主張せず、caller-supplied `GeneratedTextStore`をinitial seedまたは次passへ差し替えるfixture APIは提供しない。

`DisplayDocument`まではPDF非依存である。`LateResourceFinalizer`以降はprofile 1.0のPDF-specific phaseであり、`FrozenPdfResourcePlans`はPDF-readyだがbackend identity-freeである。すなわちCID/CIDToGIDMap、FontDescriptor metrics、PDF image encoding policyを持てる一方、backend handle、PDF resource name、PDF object IDは持たない。

ConfigLoader/CLIだけがdefaults、file、environment、CLI overrideを解決してimmutable `EffectiveConfig`を作る。Parserはcross-source validation済みpackageだけを`ParseOutcome::Parsed`で返す。成功variantのdiagnosticsはnote/warningだけを表せる`AdvisoryDiagnostic`に限定し、`ParseOutcome::Failed`はpackageを持たず少なくとも1件のerrorまたはfatalを持つ。他phaseもnote/warningだけがsuccess valueに同伴でき、error/fatalを1件でも持つ結果はsuccess valueを持てないtyped outcomeを使う。fatalは直ちにphaseを打ち切り、errorは安全なphase境界まで追加diagnosticを収集できるがartifact successを許さない。各phaseはimmutable inputを受け、現在時刻、system locale、system font search、random、HashMap iteration order、thread completion orderを暗黙入力にしない。

`HostAdmissionContext`はplatform-nativeなentry/project/config/resource-root pathと、それらの明示順だけを持つexecution inputである。`BuildExecutionContext`はoutput/sidecar HostPath、file/stdout sink、replace policyを別所有する。`BuildOutputCommitContext`はそれをcanonical `EffectiveConfig`へbindした全build共通のnon-cloneable session ownerであり、構築時、各write開始時、file tempのfinal atomic publish直前に全file target identityを再解決して同一file/symlink/hard-link aliasを拒否する。manifest targetが無いsessionはserializer receiptを消費してPDFだけをcommitできる。targetが有るsessionはsame-session `ManifestPublicationContext`のsealed built/failed preflightを必須とし、builtではPDF commit後にcanonical manifestをatomic publishしてから両receiptを公開する。host absolute pathを`EffectiveConfig`、canonical JSON、trace、build manifestへコピーしない。`typaxis.toml`内の`resource_roots`は専用`ConfigResourceRoot`として`EffectiveConfig`へ入り、`ProjectRoot` variantだけをwire `"."`、その他をproject root相対`PortablePath`として表す。admitted root setは各variantをhost directoryへ解決したものとexplicit CLI host rootsをそれぞれcanonicalizeし、可能ならdirectory handle化した和集合である。resource declarationの`PortablePath`を各root相対にcontained lookupし、existing regular-file candidateが0件ならnot-found、1件ならadmit、2件以上ならbytesが同一でもambiguous-path errorにする。host rootの明示順は検査とdiagnosticの順序だけを固定し、first existing candidateの選択には使わない。admission後に使用したlogical URI、bytes、hashはmanifestへ記録する。
