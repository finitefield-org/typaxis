# Expected invalid fixtures

`expected-errors.json` is authoritative. Each fixture must emit the listed
`rule_id`; `schema_rejects` states whether Draft 2020-12 must also reject it.

`rule_id` belongs to the conformance validator and is intentionally separate
from the public five-character `DiagnosticCode` wire API. A conformance runner
may map a rule failure to one or more public diagnostics, but must not serialize
`rule_id` as a `DiagnosticCode`.
