# Actions catalog

| Kind | Tier | Elevation | Reversible | In the CLI |
| --- | --- | --- | --- | --- |
| `sweep.user_temp` | Safe | No | No | Refused. The deleter is unwired. |
| `sweep.windows_temp` | Safe | Yes | No | Refused. No broker. |
| `net.flush_dns` | Safe | No | No | Not invoked. The export check does not call it. |

All three are allowed on the default autopilot list inside machine policy. Autopilot itself does not run. MCP defaults to refusing even the Safe kinds.
