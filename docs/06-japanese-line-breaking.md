# 日本語改行・禁則・字間

UAX #14のcandidateを基礎に、versioned Japanese pair tableを重ねる。

```text
PairRule(left_class, right_class)
  permission / penalty
  natural_gap / stretch / shrink
  priority
```

`loose`、`normal`、`strict`を用意し、table versionをbuild manifestへ記録する。行頭・行末禁則、連続約物、括弧、欧文語、数値単位をdata drivenに扱う。

均等割付は和文間、和欧文間、約物前後、space、inline mathを別priorityにする。emergency breakは明示opt-in、traceとdiagnostic必須。
