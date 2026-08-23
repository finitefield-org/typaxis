# Document、Style、Resource model

Semantic ASTはparagraph、heading、list、table、figure、page breakと、text、emphasis、strong、link、anchor、reference、footnote reference、line breakを表す。text contentは`TextSpan`で参照し、glyphや座標を持たない。

footnote definitionはDocument直下の別collectionに置き、reference targetを意味検証する。NodeId、footnote ID、anchor IDはdocument内で一意。

Style declarationは順序付きで、valueがtagged型を持つ。priorityは`(important, specificity, source_order, declaration_order)`の辞書式順序。style inheritanceはDAG。

ResourceCatalogはlogical font face/image宣言を持つ。layoutはlogical IDのみを使い、実bytes hashとbackend名はbuild/finalization時に確定する。
