# ADR-0142: Book /2 の実段幅で反復ヘッダーを選択する

- 状態: 実装（元 source の行グラフとヘッダー候補。安定配置・PDF は継続）
- 日付: 2026-10-07
- 関連: [設計 §14.279](../docs/28-vmb-book-production-compatibility.md#14279-実段幅で反復ヘッダーを選択する実装追補)、[ADR-0141](ADR-0141-book-2-automatic-column-pages.md)

## 問題

初期測定の最大段幅で組んだ表ヘッダーを、狭い段へそのまま反復できない。
本文と脚注の実幅に応じた別の行グラフを同時に保持し、元の表・セル・入れ子と
原リソースへの対応を検査してから、そのヘッダーの実高を容量へ予約する必要がある。

## 決定

行の収束・capture・再帰しない所有層別の再構築を、既存の単段経路と共有する。
内部の測定 frame は単段または段組の元計画を保持する。段組は専用の
BookV2ColumnLineVariantSeed と BookV2RebuiltColumnLineVariants を公開し、同時再構築する
全 seed が同じ元 column plan を参照することを検査する。既存の source・policy・resource・
native owner の同一性検査と収束結果の再照合も維持する。

段組の各行グラフから専用の source flow・表測定を構築し、元行グラフ集合への所属と
元ヘッダーの完全な source 対応を共通 collector で検査する。
BookV2ColumnTableHeaderVariant と BookV2ColumnTableHeaderCatalog は専用の測定 owner を
保持する。catalog は元 root の親幅を key に使い、元 source 階層の再測定で各 header の
実幅・start を照合する。異なる owner、重複・逆順の key を受理しない。
継続先に必要な幅の key が欠けている場合は、元 owner 付きの診断を返す。

ヘッダー付きの段組 search は各実段の幅を共通表 kernel へ渡す。脚注の表には段で
分割しない元脚注領域の幅を渡す。反復には選択した別グラフの実 header 高を使い、
元の semantic leaf は一度だけ消費する。入れ子表の独立した継続も同じ catalog へ結ぶ。

一時的な seed／header 参照配列は確保前に logical record を予約し、変換・再構築・
collector の work と受理済み record を失敗時も caller に返す。専用 owner の work には
共通処理の前に消費した変換分も含める。全 command の byte／spool 監査の完了とはしない。

## 検証と継続範囲

本文の実段幅、元脚注幅、入れ子ヘッダー・子表・rowspan・元改ページ、原 Harano と
semantic leaf の一回消費を検査する。source／plan の同一性、予算の exact／1不足と
失敗履歴、専用型から単段型への変換拒否を確認する。
結果は[実装台帳 §279](../docs/28-vmb-book-production-progress.md#book-2-column-header-variants-design-14279)へ記録する。

これは幅別のヘッダー候補と元 source のページ選択である。本文自身の選択した実段幅への
feedback、幅が収束したページ列、last_page balance、物理配置・terminal・段組 PDF と
独立検証は継続する。親ヘッダーで減る子表容量を含む共通境界の選択も継続課題であり、
今回の子表検査は独立セル継続を選ぶ容量で行う。
private driver／PDF assembly の段組拒否を維持する。
