# User config

The schema is `UserPreferences` in `castellan-core`. Unknown fields are rejected. `schema_version` must be 1.

```toml
schema_version = 1
disabled_targets = []
min_age_secs = 86400
lockdown = false
user_temp_at_sign_in = false
windows_temp_at_sign_in = false
reduce_motion = false
ascii_art = false
```

`min_age_secs` cannot be below 3600. Missing wish fields default to false, so an older file still parses. Unknown fields are rejected.

The keep loads this file from `{LOCALAPPDATA}\Castellan\preferences.toml` when the TUI opens, and writes it when a setting changes. `user_temp_at_sign_in` and `windows_temp_at_sign_in` record a wish. They do not register a scheduled task, and `decide` does not treat them as permission to delete. `doctor` reads the path and does not create the file.
