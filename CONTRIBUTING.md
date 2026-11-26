# Contributing

1. Make your change on a branch created from `dev`.
2. Run `cargo fmt`.
3. Open a pull request against `dev`, not `main`.

CI (clippy, tests, and builds on every supported platform) must pass before a pull request can be merged.

## Branches

- `dev` is where changes are collected. Nightly builds come from it.
- `main` is for releases only. The maintainer merges `dev` into `main` once enough changes have accumulated.
