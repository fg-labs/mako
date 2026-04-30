# Contributing to mako

Thanks for your interest in contributing! `mako` is intentionally
small — a thin CLI wrapper around the sort engine from
[fgumi](https://github.com/fulcrumgenomics/fgumi). Most
correctness and performance work happens upstream in fgumi.

## Where to file what

- **Sort engine bugs** (records out of order, sort crashes, performance
  regressions, header tag handling): file against
  [fulcrumgenomics/fgumi](https://github.com/fulcrumgenomics/fgumi).
- **mako-specific issues** (CLI flags, help text, packaging, install
  instructions, release tarballs, version banner): file here.

If you're not sure, file it here and we'll route it.

## Development setup

### Prerequisites

- Rust toolchain (pinned via `rust-toolchain.toml`)

### Install git hooks

We use a pre-commit hook to enforce formatting and lint cleanliness:

```sh
./scripts/install-hooks.sh
```

The hook runs `cargo ci-fmt` and `cargo ci-lint` before each commit.

### Running checks manually

```sh
# Format check (fails if formatting differs)
cargo ci-fmt

# Lint check (fails on any warnings)
cargo ci-lint

# Run all tests
cargo ci-test
```

### Bypass hooks (use sparingly)

```sh
git commit --no-verify -m "message"
```

## Code style

- Run `cargo fmt` before committing.
- Fix all clippy warnings.
- Follow [Conventional Commits](https://www.conventionalcommits.org/)
  for commit messages — the changelog is generated from them.

## Pull requests

1. Ensure all CI checks pass (`cargo ci-fmt`, `cargo ci-lint`,
   `cargo ci-test`).
2. Keep PRs focused.
3. Include tests for new behavior.
4. Update `CHANGELOG.md` (or rely on `release-plz` to generate it from
   conventional commits).
