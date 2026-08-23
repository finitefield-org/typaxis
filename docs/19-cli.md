# CLI

```text
typaxis build INPUT -o OUTPUT [--trace TRACE.json] [--emit-build-manifest MANIFEST.json]
typaxis check INPUT
typaxis dump-ast INPUT --format json
typaxis dump-layout INPUT --page N
typaxis inspect-font FONT
typaxis list-fonts --font-dir DIR
```

CLI tokenが正確に`OUTPUT=-`ならPDF bytesをstdoutへ出す。build manifestはhost pathを持たず、stdoutなら`output.sink = "stdout"`、その他のHostPathなら`output.sink = "file"`を記録する。したがって`./-`は通常fileとして扱える。traceとbuild manifestは常に明示されたsidecar HostPathへ出し、stdout/stderrへ混在させない。`--trace PATH`と`--emit-build-manifest PATH`はpath argument必須で、PDF stdout時にもhost file pathを指定すれば併用できる。

`dump-layout --page N`の`N`はCLI利用者向けの1-based physical page numberで、`N >= 1`を要求する。内部・canonical JSONの0-based `page_index`へはchecked `N - 1`で変換する。

build option: config、repeatable resource-root、strict、trace PATH、trace-text、emit-build-manifest PATH、max-<limit> override、no-compress、force。`--trace-text`は`--trace PATH`指定時だけ有効。profile 1.0は常にdeterministicであり、determinismを無効化するoptionを持たない。

`--config HOST_PATH`はpartial raw config fileを選び、`--resource-root HOST_DIR`は順序付き`HostAdmissionContext`へ追加する。どちらもcanonical EffectiveConfig fieldではない。config file内の`resource_roots`は`ProjectRoot`（wire `"."`）または`Relative(PortablePath)`からなる`ConfigResourceRoot` setで、EffectiveConfig/hash対象のunique UTF-8-byte-sorted arrayである。admitted rootsは各variantをproject rootから解決したdirectoryとexplicit CLI rootsをcanonicalize/handle化した集合である。host root順は検査/diagnostic順であり、同じdeclaration `PortablePath`が複数rootに存在すればambiguous-path errorとしてfirst existing fileを選ばない。`allowed_uri_schemes`も同じcanonical set規則を使う。canonical optionのprecedenceはbuilt-in defaults、config file、`TYPAXIS_` environment、CLIの順に後勝ち。CLIでcanonical fieldを上書きするのは`--strict`、`--no-compress`、`--max-<limit>`だけで、output/trace/force等はbuild execution optionとして分離する。

exit code: 0 success、1 input/layout diagnostic、2 usageまたはunknown command/option、3 I/O、4 internal invariant、5 resource limit。strict pagination fallbackは1でPDFを生成しない。

file outputとsidecarは各targetと同directoryのtemporary fileへwrite/fsyncする。file output、trace、manifest targetはcanonicalized parent+leafと、既存targetならplatform file identityでも比較し、symlink aliasを含む同一targetを`--force`の有無にかかわらずwrite前のusage errorにする。さらに各write開始時とtemp publish直前に全target identityを再解決し、CLI admission後またはtemp write中にsymlink/hard-link aliasへ変化した場合もatomic publish前にfail closedにする。`--force`なしではatomic no-replace primitiveでcommitし、存在check後の通常renameでrace上書きしない。`--force`時だけatomic replaceを使う。CLI parseとEffectiveConfig検証後はmanifest optionに依存しないnon-cloneable `BuildOutputCommitContext`が成立し、`--emit-build-manifest`指定時だけ同じoutput sessionへbindしたnon-cloneable `ManifestPublicationContext`を追加する。manifest無しbuildはserializer receiptのpreflight後にself-consuming PDF単独commitができ、manifest targetが有るbuildは完全なsealed built-manifest preflightを必須として単独経路を拒否する。manifest context成立前のusage/config failureではmanifestを変更しない。成立後のvalidation/layout/PDF failureではcleanup後に`status = failed`、`output = null`をsealed atomic publisherが発行し、確定済みならlayout summary、admit済みならcanonical partial input/resource factsを含める。atomic publish前のmanifest publication failureはexit 3で既存sidecarを保持する。built file outputでは同じterminal APIがPDF commit後にcanonical manifest bytesをtemp write/fsync/atomic commitする。manifest pre-publication errorはPDF sink receiptを保持し、directory syncだけがpublish後に失敗したerrorはvisibleなcomplete publicationを保持するため、実在するartifactをrollback扱いしない。PDF stdout時はbytes/hash/page/object factsをstreaming集計し、全write成功後だけbuilt manifestをpublishする。stdout write失敗はfailed/output-nullで、部分streamをbuilt扱いしない。stdout成功後のmanifest失敗では送信済みPDFをrollbackできない。stdoutにはPDF bytes以外を書かず、diagnosticはstderrへ出す。

INPUT、exact `-`以外のOUTPUT、config、resource-root、FONT、DIR、sidecar pathはplatform-native `HostPath`でabsolute pathを許す。source/resource declaration、includeなどpackage内で永続化する値だけがslash-separated relative `PortablePath`である。host absolute pathをcanonical JSONやbuild manifestへ記録せず、outputのhost非依存なsink分類だけをmanifestへ記録する。

## Configuration and environment prefix

The default project configuration file is `typaxis.toml`. Environment overrides use the `TYPAXIS_` prefix. The source extension `.tsf` means **Typaxis Source Format**.

Raw `typaxis.toml` requires the exact contract ID but may omit fields supplied by defaults. Unknown keys are errors. Environment keys use upper snake case and `__` between nested components, for example `TYPAXIS_LIMITS__MAX_INPUT_BYTES`; values use the target field's TOML scalar/array syntax. Unknown `TYPAXIS_` keys, non-UTF-8 values, and invalid typed values are usage/config errors and are not ignored.

`--max-ast-nesting-depth N`はpositiveかつprofile maximum 64以下でなければならない。Document rootとroot StyleRuleをdepth 1とし、typed child edgeとvalid `extends` edgeごとに1増やす。exact `N`は許可し、`N + 1`はrecursive validation、indexing、fingerprinting、cascadeを始める前にresource-limit errorとして拒否する。

`--max-fonts N`はpositiveかつsix-uppercase-letter subset tag namespace `26^6 = 308,915,776`以下でなければならない。exact maximumは許可し、max+1はresource admissionやsubset allocationを始める前のEffectiveConfig validationで拒否する。
