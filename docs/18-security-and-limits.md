# Securityとresource limits

全limitを`ResourceLimits`、TOML Schema、CLI overrideで同期する。checked arithmetic、allocation前検査、parser nesting、include、font/image decode、fragment、pass、page、spool、object、output bytesを制限する。

include/font/image pathは許可root内で解決する。URL fetchは初期版禁止。symlink解決後root外を拒否し、可能なplatformではdirectory handle相対openでTOCTOUを減らす。

font/image parserはuntrusted。unsafeは禁止し、external decoder結果もdimensionsとdecoded bytesを再検査する。PDF name/string/XML/URIは専用encoder/validatorだけを通す。
