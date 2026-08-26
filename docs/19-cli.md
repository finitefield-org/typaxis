# CLI

> **現行input status:** `build`、`check`、`dump-ast`、`dump-layout`の`INPUT`は、下記のbounded reference TSFである。DocumentPackage JSONは別の公開`build-package`/`check-package` commandへ入力し、`build`はJSONをsniffしない。supported reference TSFでは`dump-ast --format json -> build-package` round tripが成立する。normativeなproducer contractは[docs/26](26-machine-input-cli.md)を参照する。

```text
typaxis help [COMMAND]
typaxis --version
typaxis build INPUT -o OUTPUT [--trace TRACE.json] [--emit-build-manifest MANIFEST.json]
typaxis build-package PACKAGE -o OUTPUT [--package-root DIR] [--profile PROFILE]
  [--resource-root DIR ...] [--trace TRACE.json] [--trace-text]
  [--emit-build-manifest MANIFEST.json] [--emit-diagnostics DIAGNOSTICS.json]
typaxis check INPUT
typaxis check-package PACKAGE [--package-root DIR] [--profile PROFILE]
  [--resource-root DIR ...] [--emit-diagnostics DIAGNOSTICS.json]
typaxis capabilities --format json
typaxis dump-ast INPUT --format json
typaxis dump-layout INPUT --page N
typaxis inspect-font FONT
typaxis list-fonts --font-dir DIR
```

## Delivery status axes

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| current `build` reference TSF | Yes, current 1.1 | Yes, bounded reference subset | Yes | No |
| DocumentPackage Schema / `dump-ast` export | Yes, current 1.1 plus frozen 1.0 input | Yes | Yes, package round trip | Yes, M1 host gate |
| sealed machine package commands | Yes, ADR-0027 | Yes | Yes, macOS/Linux fixture gate | Yes, M1 host gate |
| `typaxis.machine-pdf/paragraph-1` | Yes, closed profile | Yes | Yes, macOS/Linux combined PDF/sidecars | Yes |

`Contract-defined`やoffline Schema validationはcommand registrationを意味しない。上表のpublic E2Eはclean-built binaryのpositive/negative fixtureで確認済みであり、M1 release statusは同一source/artifactに対するLinux/macOS actual-host evidenceの集約で閉じた。

## Public machine command contract

[ADR-0027](../adr/ADR-0027-machine-document-package-ingestion.md)に従い、次のgrammarをtop-level parser、dispatch、helpへ登録している。各optionの完全な一覧は`typaxis help build-package`、`typaxis help check-package`、`typaxis help capabilities`で確認できる。

```text
typaxis build-package PACKAGE.json -o OUTPUT.pdf \
  [--package-root DIR] \
  [--profile typaxis.machine-pdf/paragraph-1] \
  [--config CONFIG] [--resource-root DIR ...] \
  [--strict] [--no-compress] [--max-<limit> N ...] \
  [--trace TRACE.json] [--trace-text] \
  [--emit-build-manifest MANIFEST.json] \
  [--emit-diagnostics DIAGNOSTICS.json] [--force]

typaxis check-package PACKAGE.json \
  [--package-root DIR] \
  [--profile typaxis.machine-pdf/paragraph-1] \
  [--config CONFIG] [--resource-root DIR ...] [--max-<limit> N ...] \
  [--emit-diagnostics DIAGNOSTICS.json]

typaxis capabilities --format json
```

- `build`はreference TSF、`build-package`はDocumentPackageであり、extension/content sniffingでmodeを切り替えない。
- `--package-root`省略時はPACKAGEのlexical parentを使い、明示時はPACKAGE自体のcontainmentをopen前に検査する。canonical artifactへabsolute HostPathを保存しない。
- companion sourceはpackage rootだけから解決する。package rootをfont/image resource rootへ暗黙追加せず、必要なら`--resource-root`またはconfigで明示する。
- M1はexactly one source、entry-only closureだけを受理し、multi-sourceや架空のinclude edgeを受理しない。
- `check-package` successはstable package/source admission、strict decode、semantic package、profile、resource metadata、computed style/font-familyまでを保証する。pagination、full glyph shaping、PDF serialization成功は保証しない。
- `check-package`は`--strict`、`--no-compress`、`--trace`、`--force`、manifest/output optionを受理して無視せずusage errorにする。
- unknown profileはusage exit 2、contained PACKAGE/resource open unavailableはPACKAGE read前`I9110`/exit 3とする。atomic publisher unavailableはpublication context構築時にtargetを変更せずexit 3とする。unsupported inputをreference TSF、別backend、rasterへfallbackしない。
- `build-package`は現行のexact `-` stdout、strict、compression、limit、alias、個別atomic publication規則を共有し、`--trace-text`は`--trace`を要求する。
- `capabilities`は`--format json`を必須とし、missing/unknown formatはusage exit 2にする。config/filesystem/ambient localeを読まず、compiled descriptorからcanonical JSONを出す。
- supported reference TSF -> `dump-ast --format json` -> `build-package`のround tripはtyped canonical JCS/DocumentFingerprintが一致することを保証し、raw JSON bytes一致を要求しない。

CLI tokenが正確に`OUTPUT=-`ならPDF bytesをstdoutへ出す。build manifestはhost pathを持たず、stdoutなら`output.sink = "stdout"`、その他のHostPathなら`output.sink = "file"`を記録する。したがって`./-`は通常fileとして扱える。traceとbuild manifestは常に明示されたsidecar HostPathへ出し、stdout/stderrへ混在させない。`--trace PATH`と`--emit-build-manifest PATH`はpath argument必須で、PDF stdout時にもhost file pathを指定すれば併用できる。

`dump-layout --page N`の`N`はCLI利用者向けの1-based physical page numberで、`N >= 1`を要求する。内部・canonical JSONの0-based `page_index`へはchecked `N - 1`で変換する。

build option: config、repeatable resource-root、strict、trace PATH、trace-text、emit-build-manifest PATH、max-<limit> override、no-compress、force。`--trace-text`は`--trace PATH`指定時だけ有効。generated page-reference textを含むpackageでtraceを要求するときはcomplete traceのため`--trace-text`も指定する。profile 1.1は常にdeterministicであり、determinismを無効化するoptionを持たない。

`--config HOST_PATH`はpartial raw config fileを選び、`--resource-root HOST_DIR`は順序付き`HostAdmissionContext`へ追加する。どちらもcanonical EffectiveConfig fieldではない。config file内の`resource_roots`は`ProjectRoot`（wire `"."`）または`Relative(PortablePath)`からなる`ConfigResourceRoot` setで、EffectiveConfig/hash対象のunique UTF-8-byte-sorted arrayである。admitted rootsは各variantをproject rootから解決したdirectoryとexplicit CLI rootsをcanonicalize/handle化した集合である。host root順は検査/diagnostic順であり、同じdeclaration `PortablePath`が複数rootに存在すればambiguous-path errorとしてfirst existing fileを選ばない。`allowed_uri_schemes`も同じcanonical set規則を使う。canonical optionのprecedenceはbuilt-in defaults、config file、`TYPAXIS_` environment、CLIの順に後勝ち。CLIでcanonical fieldを上書きするのは`--strict`、`--no-compress`、`--max-<limit>`だけで、output/trace/force等はbuild execution optionとして分離する。

exit code: 0 success、1 input/layout diagnostic、2 usageまたはunknown command/option、3 I/O、4 internal invariant、5 resource limit。strict pagination fallbackは1でPDFを生成しない。

file outputとsidecarは各targetと同directoryのtemporary fileへwrite/fsyncする。file output、trace、manifest targetはcanonicalized parent+leafと、既存targetならplatform file identityでも比較し、symlink aliasを含む同一targetを`--force`の有無にかかわらずwrite前のusage errorにする。さらに各write開始時とtemp publish直前に全target identityを再解決し、CLI admission後またはtemp write中にsymlink/hard-link aliasへ変化した場合もatomic publish前にfail closedにする。`--force`なしではatomic no-replace primitiveでcommitし、存在check後の通常renameでrace上書きしない。`--force`時だけatomic replaceを使う。CLI parseとEffectiveConfig検証後はmanifest optionに依存しないnon-cloneable `BuildOutputCommitContext`が成立し、`--emit-build-manifest`指定時だけ同じoutput sessionへbindしたnon-cloneable `ManifestPublicationContext`を追加する。manifest無しbuildはserializer receiptのpreflight後にself-consuming PDF単独commitができ、manifest targetが有るbuildは完全なsealed built-manifest preflightを必須として単独経路を拒否する。manifest context成立前のusage/config failureではmanifestを変更しない。成立後のvalidation/layout/PDF failureではcleanup後に`status = failed`、`output = null`をsealed atomic publisherが発行し、確定済みならlayout summary、admit済みならcanonical partial input/resource factsを含める。atomic publish前のmanifest publication failureはexit 3で既存sidecarを保持する。built file outputでは同じterminal APIがPDF commit後にcanonical manifest bytesをtemp write/fsync/atomic commitする。manifest pre-publication errorはPDF sink receiptを保持し、directory syncだけがpublish後に失敗したerrorはvisibleなcomplete publicationを保持するため、実在するartifactをrollback扱いしない。PDF stdout時はbytes/hash/page/object factsをstreaming集計し、全write成功後だけbuilt manifestをpublishする。stdout write失敗はfailed/output-nullで、部分streamをbuilt扱いしない。stdout成功後のmanifest失敗では送信済みPDFをrollbackできない。stdoutにはPDF bytes以外を書かず、diagnosticはstderrへ出す。

Atomic file/sidecar publication requires a registered platform committer with suitable identity, no-replace/replace, and durability primitives. The reference workspace registers that committer on Unix; other platforms fail closed with I/O exit 3 before issuing a write receipt. See `docs/16-determinism-spooling-manifest.md` for the platform boundary.

INPUT、exact `-`以外のOUTPUT、config、resource-root、FONT、DIR、sidecar pathはplatform-native `HostPath`でabsolute pathを許す。source/resource declaration、includeなどpackage内で永続化する値だけがslash-separated relative `PortablePath`である。host absolute pathをcanonical JSONやbuild manifestへ記録せず、outputのhost非依存なsink分類だけをmanifestへ記録する。

reference parserのportable source recordsは`font:<family>:<portable-path>`、`paragraph`、`text:<utf8>`、`anchor:<id>`、`reference:<id>`、`soft_break`、`hard_break`に加え、同一paragraph内のlogical site列を`inlines:text=<utf8>|reference=<id>|anchor=<id>`で表せる。inline sequenceはempty componentとunescaped `|`を拒否し、各text componentのidentity TextMapはsource内のvalue bytesだけへexact mappingされる。このbounded grammarはpage/style declarationを持たないため、物理defaultをA4、四辺20 mmのbody margin、10.5 pt font size、17 pt line heightに固定し、すべてcanonical rational PDF-point変換で導出する。

## Configuration and environment prefix

The default project configuration file is `typaxis.toml`. Environment overrides use the `TYPAXIS_` prefix. The source extension `.tsf` means **Typaxis Source Format**.

Raw `typaxis.toml` requires the exact contract ID but may omit fields supplied by defaults. Unknown keys are errors. Environment keys use upper snake case and `__` between nested components, for example `TYPAXIS_LIMITS__MAX_INPUT_BYTES`; values use the target field's TOML scalar/array syntax. Unknown `TYPAXIS_` keys, non-UTF-8 values, and invalid typed values are usage/config errors and are not ignored.

`--max-ast-nesting-depth N`はpositiveかつprofile maximum 64以下でなければならない。Document rootとroot StyleRuleをdepth 1とし、typed child edgeとvalid `extends` edgeごとに1増やす。exact `N`は許可し、`N + 1`はrecursive validation、indexing、fingerprinting、cascadeを始める前にresource-limit errorとして拒否する。

`--max-fonts N`はpositiveかつsix-uppercase-letter subset tag namespace `26^6 = 308,915,776`以下でなければならない。exact maximumは許可し、max+1はresource admissionやsubset allocationを始める前のEffectiveConfig validationで拒否する。
