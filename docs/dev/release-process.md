# Release process

There is no release yet. When the maintainer cuts one:

1. `cargo xtask ci` is green locally and on GitHub.
2. The changelog section for the version is complete, including what the version does not do.
3. Tag `vX.Y.Z` on `main`. Do not force-push the tag.
4. Attach checksums. Attach a minisign signature only after the maintainer has created the key outside this repo.
5. Authenticode waits for a certificate. Do not claim the binary is signed.
6. Do not open a winget-pkgs pull request or publish to crates.io without an explicit yes.

`publish = false` stays until that yes.
