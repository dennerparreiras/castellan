# Adding a sweep target

A sweep target is an id inside `Action::Sweep`, not a path from the user.

1. Give it a stable name (`sweep.some_target`).
2. Decide whether it needs elevation.
3. Put the allow-root behind a Win32 lookup. Do not hard-code a drive letter.
4. Extend the deny-list tests so a sibling prefix and a `..` escape are refused.
5. Document the age floor. It cannot be below 3600 seconds.
6. Keep the live deleter unwired until the handle sequence in `docs/security/safety-kernel.md` is implemented and reviewed.

Prefetch is not a candidate. See ADR 0006.
