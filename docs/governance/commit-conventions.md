# Commits

Commits follow [Conventional Commits](https://www.conventionalcommits.org/) in English.

```text
feat(guard): skip elevated hard links when the count is unknown
fix(core): reject unknown policy fields
docs(score): record the golden keep of 83
```

The scope is the crate or the area (`cli`, `guard`, `core`, `docs`, `ci`). A breaking change says `BREAKING CHANGE:` in the footer.

Add `Signed-off-by: Name <email>` (Developer Certificate of Origin). The human author is responsible for the commit, including when an agent wrote the draft.
