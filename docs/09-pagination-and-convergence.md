# Paginationと収束

通常は前方flow、page末尾の有限lookbackでbreak costを比較する。keep、widow/orphan、heading isolation、table/footnote split、unused spaceをcomponent別にtraceする。

各passは`input_fingerprint`と`output_fingerprint`を持つ。next pass inputは直前outputと一致しなければならない。

- stable: current input == current output
- cycle: current outputが以前のinput/outputに現れ、current inputとは異なる
- max-pass: stable/cycle前に上限到達

fingerprint対象はpage master、frame、fragment owner/ordinal/start/end/bounds、footnote assignment、float decision、column decision、resolved reference textを含む。Node-to-pageだけでは不十分。fallbackは文書化されたpriorityで決定し、silent convergenceを禁止する。
