# Supply chain

- Dependencies are pinned by `Cargo.lock`, which is committed.
- `deny.toml` allows a short license list, denies yanked crates, denies wildcard versions, and allows only crates.io.
- GitHub Actions are pinned by full commit SHA. See `.github/workflows/`.
- `cargo deny` runs in the `security` workflow. It is not part of `cargo xtask ci` because the local machine may not have the `cargo-deny` binary.
- No minisign private key and no Authenticode certificate are in the tree. Do not add them.
- Releases are not signed yet. [verifying-releases.md](verifying-releases.md) says so.
