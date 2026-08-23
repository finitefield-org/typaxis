# Table、footnote、column、float

初期tableはfixed/fraction column、cell paragraph、row fragmentation、repeated header。column widthはtable開始時に確定し、ページごとに変えない。oversized rowはcell内fragmentation、進捗不能ならoverflow diagnostic。

footnoteは本文仮配置、参照収集、footnote測定、reserved height更新、本文再fragmentを行う。policyはforbid/allow/force_if_oversized。

column balanceは明示指定時だけ有限候補または二分探索。float placementはhere/top/bottom/next_pageの有限候補とqueue/page carry上限を持つ。
