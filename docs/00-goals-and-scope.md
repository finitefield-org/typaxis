# 目的・非目的・品質目標

## 目的

入力bytesから再現可能なPDFを生成するRust CLIを実装する。意味構造、text変換、文字処理、段落改行、fragmentation、pagination、描画IR、resource finalization、PDF object graphを分離する。

## 品質目標

1. 同一source bytes、解決済みeffective config、admitted font/image bytes、data version、engine version、PDF profile/compression implementationからbyte-identicalなPDFを生成する。profile 1.0ではdeterminismを無効化できない。
2. 日本語横書きの禁則、和欧文間隔、均等割付をversioned data tableで扱う。
3. 検索・copy用Unicodeをcluster単位で保存し、必要時はActualTextを使う。
4. source位置と変換後text位置を混同しない。
5. 全反復、ページ数、入力展開量、spool、PDF object/output bytesを上限化する。
6. breakと収束理由をcanonical traceで説明できる。

## 非目的

縦書き、完全CSS、既存PDF編集、GUI DTP、初期版の暗号化・署名・form、任意constraint solverは対象外。

## 自作境界

組版意味論、pagination、Display List、resource binding、PDF serializerは自作する。Unicode data、font parser、shaper、compression、image decoderは抽象化し、組版判断を持たないPure Rust実装を利用できる。
