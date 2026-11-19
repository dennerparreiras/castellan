# Sweep targets

| Id | Allow root | Notes |
| --- | --- | --- |
| `user_temp` | `GetTempPath2W` | The root itself is kept. Children must be old enough and not in use. |
| `windows_temp` | `GetSystemWindowsDirectoryW` plus `Temp` | Needs elevation. The only Windows subdirectory the deny-list allows. |

Not targets: Prefetch, Recent, Windows\Temp's parent, any path the person types.
