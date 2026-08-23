# Determinism、spooling、build manifest

再現性入力はsource/config bytes、font/image bytes、Unicode/Japanese table version、shaper backend/version、engine version、PDF profile。build manifestへhashとversionを記録する。

spoolはpage recordごとにmagic、contract、length、checksumを持つ。最大page/total bytesを割当前に検査し、temp root外を削除しない。異常終了時のcleanupは作成したnonce directoryだけ。

PDF outputはtemp fileへwrite/fsyncし、成功後atomic rename。既存outputを失敗途中でtruncateしない。stdout出力ではdiagnosticをstderrに分離する。

release ZIPのtop-level directory名はcheckout directoryのbasenameから導出せず、版付きの`ARCHIVE_ROOT`定数で固定する。同じpackage bytesは展開元のdirectory名に関係なく同一ZIPになる。
