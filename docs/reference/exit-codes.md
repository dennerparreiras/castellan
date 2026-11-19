# Exit codes

| Code | Meaning | Used now |
| --- | --- | --- |
| 0 | Success, including a dry-run that deleted nothing | Yes |
| 1 | Error, including a command-line parse error | Yes. Parse errors do not yet use code 2. |
| 2 | Usage | Reserved. `cargo xtask` with no task returns 2. The `castellan` binary still returns 1 for clap errors. |
| 10 | A deducting finding at Warning or Notice | `status`, when the volume finding deducts and is not Critical |
| 11 | A deducting Critical finding | `status` |
| 20 | Policy or the dev-build gate refused the mutation | `sweep clean` |
| 21 | Elevation denied | Reserved. Nothing requests elevation. |
