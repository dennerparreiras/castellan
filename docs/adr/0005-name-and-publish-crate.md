# 0005. Name and the crates.io package

- Status: accepted
- Date: 2026-11-19

## Context

The product name is Castellan. The plan says to change it only if another Windows system tool already uses it in a way that would confuse people. The fallback name Keepwright was reserved for that case.

Checked on 2026-11-19:

- crates.io package `castellan` 0.0.3 belongs to the Mere credential keeper (`github.com/merely-made/mere`). It is not a Windows maintenance tool. `castellan-client` is also taken.
- OpenStack’s python-castellan, mycastellan.com, and the word “kastellan” are not Windows system utilities.
- `winget search` showed no Castellan package.
- The GitHub repository `dennerparreiras/castellan` did not exist yet.

## Decision

The product name stays Castellan. Keepwright is not used.

The crates.io name `castellan` is not ours. The library and binary package that could be published is `castellan-cli`, and the executable name is `castellan`. Nothing is published. `publish = false` stays on every package until the maintainer confirms a publish.

A second repository for a Scoop bucket is not created here. That needs an explicit yes from the maintainer.

## Consequences

Documentation, the UI, and the binary say Castellan. A future crates.io release will be the package `castellan-cli`, with text that points at this project so it is not mistaken for the Mere crate.
