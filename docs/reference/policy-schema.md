# Machine policy

The schema is `MachinePolicy`. Unknown fields are rejected. `schema_version` must be 1. Autopilot entries must be safe, listable kinds.

```toml
schema_version = 1
lockdown = false
autopilot_actions = ["sweep.user_temp", "sweep.windows_temp", "net.flush_dns"]
min_age_secs = 86400
sanity_max_bytes = 50000000000
sanity_max_files = 500000
mcp_allow_safe_actions = false
mcp_allow_moderate_with_approval = false
```

`mcp_allow_moderate_with_approval` does not let MCP run Moderate actions. The confirmation rules still forbid that origin. The field is reserved so a config cannot smuggle the permission in by a different name.
