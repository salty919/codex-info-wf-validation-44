# Native gate evidence (2026-08-22)

実行コマンド:

```text
cargo fmt --check
cargo check
cargo test
cargo build --release
```

結果:

- `cargo fmt --check`: PASS
- `cargo check`: PASS
- `cargo test`: PASS（lib 152、main 164、runtime 1、security 13、usage_store 36、doc-tests 0）
- `cargo build --release`: PASS

この証跡はLinux/X版の固定テストとビルドだけを示す。Windows実機導入、Windows画像、X/Windows同一SHA同等性、全異常系を代替しないため、要求台帳とリリース判定は`HOLD`のままとする。
