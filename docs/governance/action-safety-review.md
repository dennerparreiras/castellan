# Action safety review

Fill this in for any pull request that adds or changes an `Action`, a sweep target, or a tweak.

- What changes on the system, in one paragraph.
- Risk tier, and why it is Safe, Moderate, or Sensitive.
- Does it need elevation.
- How it is undone, and the test that proves the undo. If it cannot be undone (temp deletion, DNS flush), say that in the UI and in the catalog.
- What can break, including a junction inside the target and a file that is in use.
- The source that supports the benefit. A tweak without a source does not land.
- Why this is not a myth (RAM optimizers, registry cleaners, and Prefetch wipes do not land).
- Adversarial tests included.

Prefetch is already decided: it stays denied. See ADR 0006.
