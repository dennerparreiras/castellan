# Review checklist

- The change does what the description says, and the tests that prove it were actually run.
- No `unsafe` outside `castellan-winapi`. Each `unsafe` block has a `// SAFETY:` comment.
- No new way to pass a command string, a free path into a delete, or a network listener.
- Mutations still go through `decide` and `decide_node`.
- Debug builds still refuse a real root. Tests do not set `CASTELLAN_ALLOW_REAL_SYSTEM`.
- Docs and the changelog match the behavior, including what is still refused.
- No secrets, no real machine dumps, no files from the maintainer's personal script folder.
