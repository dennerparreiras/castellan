# Privilege model

The process in this build runs as the user who started it (`asInvoker`, once a manifest exists; today there is no embedded manifest). It does not start an elevated child.

Windows temp is typed as an elevated action and is not performed. See [ADR 0004](../adr/0004-elevation.md).

A later broker is a separate process with a typed plan. MCP is never that process.
