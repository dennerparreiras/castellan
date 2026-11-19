# Adding a probe

A probe measures one fact and returns a `Finding`. It does not delete or "fix" anything.

1. Add the measurement type next to `VolumeSpace` if it is new.
2. Build the finding with a `FindingId` of the form `PREFIX-NNN`.
3. Cite the threshold in `docs/reference/findings-catalog.md` and in `docs/reference/health-score.md` if it affects the score.
4. Leave unassessed pillars unassessed. Do not pad the score.
5. Test the boundary (the exact percent, byte, or count where the severity changes).

`STORAGE-001` is the pattern.
