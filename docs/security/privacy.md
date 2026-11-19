# Privacy

Castellan does not phone home. There is no telemetry, no account, and no analytics sink in the code.

`status` prints volume byte counts. It does not print names of files inside Documents, Desktop, or other protected folders.

The ledger, when a future command writes it, stores action payloads. Those payloads must not contain file contents, passwords, Wi-Fi keys, or product keys. The tamper test uses a synthetic payload.

Evidence text from outside the program is capped at 512 characters and rejected if it contains control characters.
