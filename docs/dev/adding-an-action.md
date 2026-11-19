# Adding an action

Actions are enum variants in `castellan-core`. A new variant needs:

- a risk tier, an elevation flag, and a reversibility flag
- `confirmation_for` behavior (MCP and Autopilot never get Moderate or Sensitive)
- a policy test that a lower layer cannot turn it on when a higher layer forbids it
- an entry in `docs/reference/actions-catalog.md`
- the safety review in `docs/governance/action-safety-review.md`

If the variant cannot be named without a shell string, it does not belong in the catalog.
