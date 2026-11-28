# CLI reference

```text
castellan [--json] [status]
castellan about
castellan doctor
castellan sweep targets
castellan sweep clean [--dry-run]
```

`--json` affects `status`. `sweep clean` deletes nothing; see [sweep](sweep.md).

`doctor` reports that no scheduled task is registered and that sweep clean still deletes nothing.
