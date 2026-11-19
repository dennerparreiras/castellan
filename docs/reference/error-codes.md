# Error codes

| Code | Name | When |
| --- | --- | --- |
| CST-E0001 | Invalid | A value does not match its type (id, age, evidence). |
| CST-E0002 | Config | TOML or another config value was rejected. |
| CST-E0003 | Policy | Defined for policy failures. Callers today usually return a `Decision::Deny` instead of this code. |

The strings are stable. Do not renumber them.
