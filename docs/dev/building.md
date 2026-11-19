# Building

```powershell
cargo build -p castellan-cli
cargo build -p castellan-cli --release
```

The release profile uses fat LTO, one codegen unit, abort-on-panic, and stripped symbols. MSVC builds also enable the static CRT and Control Flow Guard.

The binary name is `castellan`. Debug builds print `DEV BUILD â€” real-system actions disabled` on standard error.
