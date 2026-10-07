# ADR-0141: Book /2 の段と脚注の元境界からページを自動選択する

- 状態: 実装（元 source に連続した初期ページ列。安定配置・PDF は継続）
- 日付: 2026-10-07
- 関連: [設計 §14.278](../docs/28-vmb-book-production-compatibility.md#14278-段と脚注の元境界からページを自動選択する実装追補)、[ADR-0140](ADR-0140-book-2-joint-column-page-candidates.md)

## 問題

全段と脚注の共通候補を検査できても、caller が各段の source cut を作るだけでは
元の改ページ・名前・keep・表の実継続を含む物理ページ列を選べない。後段からの参照で
脚注の必須断片が収まらない場合、先行段まで候補を戻す必要がある。

## 決定

単段の既存境界列挙と改ページ前検査を共有する。各実段の高さで元 item と表 cursor の
合法な境界を列挙し、既存の widow／orphan／heading の安定した cost 順序を使う。
sequential fill として左から右に探索し、先行段の一つの境界に続く後段の全候補を
検査してから、先行段の次の境界へ戻る。未使用の末尾段は最後に試す。
全段の cost 合計の最適化や最終ページの balance を採択した意味にはしない。

各境界の probe は元 source と demand の分岐だけを進め、脚注を確定しない。
全段の cut が揃った時点で共通候補 API を呼び、元の全ページ脚注領域へ一度だけ fit
する。必須断片が収まらなければページ全体を戻す。named retry を含む物理ページの
fit 回数を共通 reflow 上限へ数え、境界列挙にも元 lookback 上限を適用する。

探索は明示的な stack で保持し、元の最大 u16 段数に応じた再帰を作らない。
stack・request slots・ページ列を確保する前に logical record を予約し、source probe、
fit、失敗・再試行の work と受理済み record は同じ累積台帳へ残す。
これは capacity／byte／spool と全 command の予算監査を完了した証明ではない。

表は caption・cell・入れ子・rowspan の元 cursor を段間と物理ページ間へ引き継ぐ。
外側または表内の明示改ページで後段を閉じ、外側の command を一度だけ消費する。
先頭・連続・末尾の改ページで必要な空ページと、本文終了後の実脚注継続を保持する。
本文を進めずページを追加することは、この元 command と実脚注継続の場合に限る。
keep の不正な境界には元 owner の既存診断を返す。名前に応じた元 master・段数を選び、
探索中の frame・名前・診断状態は終了時に復元する。

元 source に連続した列は専用の BookV2ColumnPageSequence に保持する。
search owner・元 plan の参照・表の測定 fingerprint・候補 chain を照合し、別 search
からの利用を拒否する。単段のページ列や paint receipt への変換を提供しない。

## 検証と継続範囲

二段と共通脚注、脚注による先行段への backtracking、名前別の段数、先頭・連続・末尾の
改ページ、表 caption 内の改ページと同じページでの実表継続、keep 拒否、脚注だけの
継続を検査する。元 semantic leaf の一回消費、work／record の exact／1不足、lookback・
reflow・page 上限と失敗履歴、元 Harano、型の変換拒否を確認する。
結果は[実装台帳 §278](../docs/28-vmb-book-production-progress.md#book-2-automatic-column-pages-design-14278)へ記録する。

この列は初期測定に基づく source 選択である。選択した実段幅への再組版 feedback、
反復 header の幅 variant、last_page balance、安定した物理配置・terminal・段組 PDF と
独立検証は継続する。private driver／PDF assembly の段組拒否は維持する。
