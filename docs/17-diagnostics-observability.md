# Diagnosticsとtrace

Diagnostic codeは安定API。category:

- P1xxx parser/source
- T2xxx text map
- S3xxx style/document
- F4xxx font/shaping
- L5xxx line/block layout
- G6xxx pagination
- R7xxx resource finalization
- D8xxx PDF
- I9xxx limits/security

source spanとtext spanを別fieldで持つ。traceはversioned canonical JSON。本文全文やfont bytesは既定で含めず、textはID/span/hashと短いescaped excerpt。`--trace-text`のみopt-in。
