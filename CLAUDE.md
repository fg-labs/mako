# CLAUDE.md — mako

Project-local guidance for AI agents working in this repo.

## What this project is

`mako` is a single-binary CLI tool that sorts SAM/BAM files. It is a
thin wrapper around the sort engine in
[fgumi](https://github.com/fulcrumgenomics/fgumi). The only original
code in this repo is `src/main.rs` (clap entrypoint, mimalloc allocator,
version banner) and `tests/cli.rs` (end-to-end CLI tests).

## Hard rules

1. **Do not add original sort code to this repo.** All sort logic lives
   in fgumi (`fgumi_lib::sort`, `fgumi_lib::commands::sort::Sort`). If
   sort behavior, performance, or correctness needs to change, the
   change happens in fgumi upstream — open a PR there, then bump
   `fgumi` in `Cargo.toml` here.
2. **Do not add subcommands or other tools.** mako is intentionally
   flat: one binary, one job (sort). Resist scope creep.
3. **Do not vendor fgumi or use a git submodule.** mako depends on
   `fgumi` from crates.io as a normal versioned dependency. This is
   required for crates.io to accept the published artifact.

## Where to make changes

- CLI flag wiring, help text, version banner → `src/main.rs`.
- End-to-end tests → `tests/cli.rs`.
- Bumping fgumi → `Cargo.toml` (one-line version bump).
- Release machinery → `release-plz.toml`, `cliff.toml`,
  `.github/workflows/publish.yml`.
- CI → `.github/workflows/check.yml`.

## Local checks before committing

```sh
cargo ci-fmt
cargo ci-lint
cargo ci-test
```

The pre-commit hook (installed via `scripts/install-hooks.sh`) runs the
first two automatically.

## Conventions

- Conventional commits (`feat:`, `fix:`, `chore:`, etc.). The changelog
  is generated from them.
- Keep `tests/cli.rs` self-contained — generate test BAMs
  programmatically with `noodles` (dev-dep). Do not commit test data.
- Toolchain is pinned via `rust-toolchain.toml`. Do not bump casually.
