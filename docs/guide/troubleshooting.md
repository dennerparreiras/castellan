# Troubleshooting

**The program says it is a dev build.** You are running a debug binary. Real deletions stay off.

**`sweep clean` exits 20.** That is this build working as designed.

**Status shows no band.** Only part of the keep was measured. The number may still be printed as a partial score. The band stays hidden below 500 thousandths of confidence.

**Clippy fails the build.** `cargo xtask ci` treats warnings as errors. Fix the warning. Do not allow it away to go green.
