# Hardening checklist

For a change that can mutate the machine:

- [ ] Action is a closed enum variant.
- [ ] Policy tests show a lower layer cannot loosen a higher one.
- [ ] Path tests include a sibling prefix and a `..` escape.
- [ ] Reparse points are not entered.
- [ ] Elevated hard links fail closed when the link count was not read.
- [ ] The CLI path used in tests does not set `CASTELLAN_ALLOW_REAL_SYSTEM`.
- [ ] Docs say whether the action is reversible.

This build's clean command stays on the refused side of that list on purpose.
