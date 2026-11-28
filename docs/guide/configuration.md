# Configuration

User preferences and machine policy have schemas ([config](../reference/config-schema.md), [policy](../reference/policy-schema.md)). The keep reads and writes `{LOCALAPPDATA}\Castellan\preferences.toml` when you change a setting. `castellan doctor` reports that path when the file exists and does not create it.

Sign-in cleanup flags are wishes. `decide` ignores them. Saving the file does not register a scheduled task and does not delete anything. A lower layer can only tighten policy. The age floor cannot go below 3600 seconds. `CASTELLAN_ASCII` and `CASTELLAN_NO_ANIM` force those modes on for the session. They do not force them off. `NO_COLOR` stays an environment flag.
