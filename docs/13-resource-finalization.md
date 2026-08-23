# Deterministic resource finalization

Display pagesをspoolしながらlogical useを収集し、次の順でfinalizeする。

1. input/resource bytes hashを確定。
2. font glyph closureとimage decode profileを確定。
3. resourceを`(kind, content_hash, logical_id)`でstable sort。
4. subset GID、CID、backend handleを固定。
5. PDF resource nameとobject IDをstable sort後に付与。
6. frozen resource manifestをPDF backendへ渡す。

HashMap iteration、parallel completion、file discovery orderから番号を決めない。same original GIDが複数Unicode clusterを表してもsubset GIDは共有できるがCID/ActualTextはcluster extraction planに従う。
