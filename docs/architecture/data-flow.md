# Data flow

Status:

1. The CLI asks `castellan-winapi` for the Windows directory and its free space. That is a read.
2. `castellan-engine` builds `STORAGE-001` and a score over the Storage pillar only.
3. The CLI prints the persona line and the partial score, or JSON. `display_band` is absent because confidence is below 500.

Sweep clean:

1. The CLI evaluates `mutation_allowed` with the real-system flag forced off and the sandbox flag forced off.
2. It prints that nothing was deleted and returns. No walker runs against the disk.
