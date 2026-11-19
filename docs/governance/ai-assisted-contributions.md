# AI-assisted contributions

Agents and other AI tools may draft code. The human who opens the pull request is the author. They must understand the diff, run the tests, and say which tool they used.

The same gates apply: `cargo xtask ci`, the safety review, and no real mutations on a development PC. An agent must not generate a minisign key, publish a crate, or create a second GitHub repository.
