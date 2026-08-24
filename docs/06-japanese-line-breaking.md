# 日本語改行・禁則・字間

UAX #14のcandidateを基礎に、versioned Japanese pair tableを重ねる。

```text
PairRule(left_class, right_class)
  permission / penalty
  natural_gap / stretch / shrink
  priority
```

`loose`、`normal`、`strict`を用意し、table versionをbuild manifestへ記録する。行頭・行末禁則、連続約物、括弧、欧文語、数値単位をdata drivenに扱う。

Profile 1.0のregistered handle `typaxis-jlreq-horizontal/1.0.0`は`JapaneseLineBreakMode`と`japanese_pair_rule`で実体化する。tableはopening/closing punctuation、small kana、nonstarter、和文、Latin、numeric、spaceをclosed classへ分類し、UAX #14 candidateを追加せずに禁則candidateだけを除去する。`loose`はsmall kana/nonstarter前をUnicode判断のまま残し、`normal`/`strict`は禁則にする。和欧文pairは1/1024-em単位のnatural/stretch/shrink、priority、mode別penaltyを返すため、font sizeやhost localeをtable lookupへ暗黙入力にしない。

均等割付は和文間、和欧文間、約物前後、space、inline mathを別priorityにする。emergency breakは明示opt-in、traceとdiagnostic必須。
