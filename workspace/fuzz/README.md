# Fuzzing

The fuzz package is excluded from the release workspace so normal stable-Rust
gates do not link libFuzzer. Run either bounded target with a nightly toolchain:

```console
cargo install cargo-fuzz
cargo +nightly fuzz run unicode_linebreak -- -max_total_time=60
cargo +nightly fuzz run reference_parser -- -max_total_time=60
```

Both targets accept arbitrary bytes, discard invalid UTF-8 at the syntax
boundary, and exercise only public bounded entrypoints.
