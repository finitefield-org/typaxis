# ADR-0007: 収束をinput/output fingerprint chainで判定する

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

state 0を選択不能なseedとし、pass iがmaterialized state i+1を生成する。stable、cycle、max-passを区別して必ず停止する。cycle/max-passではstate 1..pass_countを`(hard_violation_count,total_cost,page_count,state_index)`で辞書式最小化する`lowest_cost_then_earliest` policyを使い、warningを必須、strict時はerror/no PDFとする。

## Consequences

- Rust、Schema、fixture、docs、validatorを同時更新する。
- 型で表現できない外部入力は境界validatorで拒否する。
