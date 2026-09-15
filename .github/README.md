# Continuous integration

The hosted CPU workflow checks formatting, Clippy, document/schema/dependency
contracts, Python tool tests and workspace CPU tests. GPU tests are explicitly
ignored there, not reported as GPU passes. The feature matrix includes pure glTF,
pure asset tooling and optional profile/schema configurations.

The GPU workflow is a manual dispatch against the repository default branch on a
trusted `self-hosted, linux, stylized-gpu` runner. It executes the same strict GPU
command as local development. No runner has been provisioned by this change;
local qualification must not be described as active hosted GPU coverage.

Missing GPU, source assets or accepted baselines fail a required task. External
Genshin preparation/qualification is explicit and its art is not uploaded as a
public CI artifact. Refer to CONTRIBUTING.md for the isolated engineering gate.
