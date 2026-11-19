# Undo and the ledger

Temp deletion and DNS flush cannot be undone. The ledger is an append-only hash chain so a record can be checked later. The CLI does not write records yet, because it does not perform those actions.

`Ledger::verify` fails closed when a stored row is altered. The test covers that with a database under the temp directory.
