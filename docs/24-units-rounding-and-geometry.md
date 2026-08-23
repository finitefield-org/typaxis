# Units、rounding、geometry

Internal length unit is `pdf_point_1_65536`: one integer step equals `1 / 65536` PDF point, and one PDF point equals `1 / 72 inch`.

Unit parsing uses exact rational factors:

- pt = 1
- in = 72
- mm = 360 / 127
- cm = 3600 / 127
- px, when enabled, = 3 / 4 pt

Final integer conversion is round-half-to-even. Addition/subtraction use checked i64; multiplication/division and matrix composition use checked i128 intermediate before rounding back.

Affine transform:

```text
a b c d : signed 16.16 dimensionless
 e f     : Length
```

Rect x/y may be negative, width/height must be positive. Page width/height must be positive. Geometry constructors enforce these invariants; public callers cannot construct negative extents directly.

A4は`210 mm × 297 mm`なので、exact rational testはそれぞれ`210 × 720 / 254 pt`と`297 × 720 / 254 pt`を使用する。`72 / 254`は10倍不足するため禁止する。
