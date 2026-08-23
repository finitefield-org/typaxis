# CLI

```text
typaxis build INPUT -o OUTPUT
typaxis check INPUT
typaxis dump-ast INPUT --format json
typaxis dump-layout INPUT --page N
typaxis inspect-font FONT
typaxis list-fonts --font-dir DIR
```

build option: config、resource-root、strict、deterministic、trace、emit-build-manifest、max-layout-passes、no-compress、force。

exit code: 0 success、1 input/layout diagnostic、2 usage、3 I/O、4 internal invariant、5 resource limit。

file outputは同directoryのtemporary fileからatomic replace。`--force`なしで既存fileを上書きしない。PDF stdout時はstdoutにPDF bytes以外を書かない。

## Configuration and environment prefix

The default project configuration file is `typaxis.toml`. Environment overrides, when implemented, use the `TYPAXIS_` prefix. The source extension `.tsf` means **Typaxis Source Format**.
