# Migrating from the old scripts

The scripts this program replaces deleted temp files, flushed DNS, reset Winsock at boot, poked Windows Update, and copied fonts. Several of those steps do harm or do nothing useful. Castellan does not reimplement them line for line.

| Old step | Castellan |
| --- | --- |
| Delete the user temp directory and `Windows\Temp`, including new files and open files | A sweep target, once the live deleter exists. Age floor 1 hour, default 24 hours. In-use files stay. The allow-root stays. This build refuses the delete. |
| Delete Prefetch | Not available. Prefetch is denied. See [ADR 0006](../adr/0006-prefetch-is-hard-denied.md). |
| Delete Recent | Not a default target. It will stay opt-in if it is added, and it is not in this build. |
| `ipconfig /flushdns` | Catalog action `FlushDns`. Not called by the CLI yet. |
| `ipconfig /registerdns` | Not in the default set. |
| `netsh winsock reset` at boot | Not an action. |
| `wuauclt` | Not an action. |
| `del` of a path typed into a batch file | Not possible. There is no command string. |
| Restart the audio service | Not in this build. When it exists, the order is `AudioEndpointBuilder`, then `Audiosrv`. |
| DISM, SFC, chkdsk | Not in this build. They need the elevated broker and a progress parser. They will not run on the maintainer's PC from the test suite. |
| Copy fonts into the user font folder | Not in 1.0. It is a later note, not a sweep. |

The batch files themselves are not in this repository.

## What you can run today

From a build directory:

```text
castellan status
castellan status --json
castellan doctor
castellan sweep targets
castellan sweep clean --dry-run
```

`sweep clean` without `--dry-run` exits 20 and deletes nothing. That is the intended behavior of this build.
