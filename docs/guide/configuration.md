# Configuration

User preferences and machine policy have schemas ([config](../reference/config-schema.md), [policy](../reference/policy-schema.md)). This build does not read those files from disk. The compiled defaults are what `decide` uses when a caller passes `compiled_default()`.
