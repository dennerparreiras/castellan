# Security policy

Castellan is built to run maintenance actions on a Windows PC. A bug in the safety kernel can delete the wrong file or weaken a control. Please report those privately.

## Reporting

Use [GitHub Private Vulnerability Reporting](https://github.com/dennerparreiras/castellan/security/advisories/new) once the repository has that feature turned on. Until then, email [dennerparreiras@gmail.com](mailto:dennerparreiras@gmail.com) with the subject `Castellan security`.

Include the version (`castellan about`), the command you ran, and what you expected. Do not include passwords, product keys, Wi-Fi credentials, or the contents of personal files.

## What we consider in scope

- Deleting or modifying a path outside the allow-root.
- Following a junction, symlink, or mount point during a delete.
- An action that is not in the closed catalog, including any way to pass a shell string.
- A debug build mutating the real system without the explicit flag.
- MCP or any other listener accepting a connection, or a sensitive action running without a person present.
- Weakening Defender, firewall, UAC, SmartScreen, VBS, HVCI, or Windows Update.
- Secrets committed to the repository.

## What we do not consider a vulnerability by itself

- A low Keep Score.
- A finding you disagree with, when the documented threshold was applied.
- The absence of a feature that `docs/dev/STATUS.md` already lists as not built.

## Supported versions

| Version | Supported |
| --- | --- |
| 0.1.x (this foundation) | Yes, best effort |
| Anything before the first tagged release | The default branch |

There is no signed release yet. Do not treat an unsigned build as a finished security product.
