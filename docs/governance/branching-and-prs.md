# Branches and pull requests

Trunk-based. `main` is the only long-lived branch. Short branches:

- `feat/â€¦`
- `fix/â€¦`
- `docs/â€¦`
- `chore/â€¦`
- `sec/â€¦`

Every change lands through a pull request. Do not push directly to `main`. Do not force-push `main`. Squash merge.

A pull request says what changed and why, how it was tested (commands and the result), the security impact, and which docs moved with the code. TUI changes include a capture. One subject per pull request.

Reviews of `castellan-guard`, `castellan-winapi`, `packaging/`, `.github/`, and `docs/security/` need the maintainer. `CODEOWNERS` records that.
