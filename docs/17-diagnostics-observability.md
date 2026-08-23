# Diagnosticsとtrace

Diagnostic codeは安定API。category:

- P1xxx parser/source
- T2xxx text map
- S3xxx style/document
- F4xxx font/shaping
- L5xxx line/block layout
- G6xxx pagination
- R7xxx resource finalization
- D8xxx display/PDF
- I9xxx limits/security

severityは`note`、`warning`、`error`、`fatal`の閉じた集合である。`AdvisoryDiagnostic`は型としてnote/warningだけを許し、phaseのsuccess valueに同伴できるdiagnosticもこれだけである。errorまたはfatalを1件以上含むphase outcomeはfailureで、success valueおよびartifact successを持たない。fatalは安全な後処理を除いて即時打切りとし、errorは安全なphase境界まで複数件を収集してよいが成功へ降格しない。

source spanとtext spanを別fieldで持つ。traceはversioned JSON valueをRFC 8785 JCS UTF-8 bytesへencodeする。本文全文やfont bytesは既定で含めず、textはID/span/hashと短いescaped excerpt。`--trace-text`のみopt-in。

pagination cycle/max-pass fallbackはG6xxx warningで、termination reason、全候補score、selected state、policy IDを必ず持つ。strict modeでは同codeをerrorへ昇格し、PDF successを返さない。
