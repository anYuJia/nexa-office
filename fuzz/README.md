# Nexa OOXML fuzz targets

These targets are outside the main workspace so normal builds do not pull in libFuzzer.

```bash
cargo install cargo-fuzz
cd fuzz
cargo fuzz run opc_xml
cargo fuzz run zip_package
```

The targets use stricter resource limits than product defaults to keep fuzz iterations bounded.
