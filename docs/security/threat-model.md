# Threat model

Method: STRIDE, scoped to the 0.1.0 slice. Later processes (broker, MCP, scheduled task) are in the model as planned trust boundaries, and the mitigations that are not built yet are marked.

## Assets

- The person's files, including anything outside an allow-root.
- Windows security settings: Defender, firewall, UAC, SmartScreen, VBS/HVCI, updates.
- The Castellan ledger and policy files, once they live on disk outside a test.
- The person's attention: a fake score or a scare screen is itself a harm.

## Actors

- The person at the keyboard.
- A local process running as the same user, which can write the user profile, `%TEMP%`, and the user config.
- A local process that is not the user, which can write world-writable directories and plant junctions.
- A later MCP client on stdio. It is not a network peer. There is no listener in this build.

Castellan does not accept work from the network. The default build opens no socket.

## Boundaries

```text
person / MCP client / CLI args
        |
        v
  castellan-cli  -- read-only Win32 for status
        |
        v
  castellan-core decide()     (pure)
        |
        v
  castellan-guard             (pure decision + in-memory walker)
        |
        v
  castellan-winapi            (unsafe, not called to delete in this build)
```

The elevated broker is not in the process tree yet. Drawing it as if it were would hide the gap. See [STATUS](../dev/STATUS.md).

## STRIDE

| Threat | Property | In this build |
| --- | --- | --- |
| Spoofing a policy file | Authenticity | Machine policy is parsed with `deny_unknown_fields` and can only tighten. Nothing is loaded from disk yet; callers pass structs. When a file is loaded, an untrusted directory must not be able to widen the compiled rules. |
| Tampering with a path between check and delete | Integrity | The in-memory tests decide on a final path. The live deleter is unwired. Production delete has to be by handle after `open_no_follow` and a final-path check. |
| Repudiation of a mutation | Accountability | The store hashes each ledger row to the previous row with BLAKE3. The CLI does not write a ledger row because it does not mutate. |
| Information disclosure | Confidentiality | Status reports byte counts, not file names from the user's documents. Evidence text rejects control characters and is capped. No telemetry. |
| Denial of service | Availability | Autopilot sanity caps abort a large plan when no person is present. A hostile child name is skipped rather than joined. |
| Elevation of privilege | Authorization | There is no elevated child. Windows temp is typed as needing elevation and cannot be performed. MCP cannot run Moderate or Sensitive actions, and Safe MCP actions default to off. |

## Abuse cases the tests cover

- `..` that leaves the temp allow-root and lands in Documents is denied.
- `D:\Tempest` is not inside `D:\Temp`.
- A reparse point is not entered. Its children in the fake tree are not deleted.
- A final path outside the allow-root is not deleted, even if a directory listing named it.
- A child named `..` or a name containing a separator is not joined.
- Lockdown on either layer denies. A lower layer cannot shorten the minimum age.
- An unknown policy field is rejected.
- A debug build refuses a root that is not a strict child of temp.
- The read-only live scan creates one temp file, sees it, and leaves it for the test to delete. It does not walk into a reparse point.

## Abuse cases not covered yet

- A real junction planted in a temp directory on Windows. Creating one on the maintainer's PC is out of bounds; the CI runner does not have this test yet.
- TOCTOU against a live directory while a handle-based deleter runs. The deleter does not exist.
- A malicious MCP client. The server does not exist.
- IPC to a broker. The broker does not exist.
- Hard-link count read from `FILE_STANDARD_INFO`. Callers must pass `u32::MAX` until that read exists, and the elevated branch then skips the file.

## Out of scope

- Malware that is already running as the user can already delete the user's files without Castellan.
- Physical access to the machine.
- A compromised compiler or a compromised crates.io crate. That is supply chain, tracked in [supply-chain.md](supply-chain.md).
