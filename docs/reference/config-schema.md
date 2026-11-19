# User config

The schema is `UserPreferences` in `castellan-core`. Unknown fields are rejected. `schema_version` must be 1.

```toml
schema_version = 1
disabled_targets = []
min_age_secs = 86400
lockdown = false
```

`min_age_secs` cannot be below 3600. The file is not loaded from disk in this build. Tests construct the struct or parse a string.
