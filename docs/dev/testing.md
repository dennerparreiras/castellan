# Testing

```powershell
cargo xtask ci
```

That is fmt, clippy with warnings denied, `cargo test --workspace`, and the architecture scan.

The suite uses temp directories and in-memory filesystems for anything that deletes. Read-only Win32 checks (known folders, free space, the DNS export) may look at the machine. They do not flush DNS and do not delete outside the temp directory the scan test itself created.

Privileged tests are not in this tree. They belong on a disposable CI runner when the broker exists.
