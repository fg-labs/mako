# Changelog

All notable changes to this project will be documented in this file.

## [0.1.3] - 2026-06-20

### Miscellaneous Tasks

- Bump fgumi to 0.3.1 and cover stdin sort (#12)

## [0.1.2] - 2026-06-11

### Bug Fixes

- Remove duplicated changelog header and unreleased placeholder (#11)

### Miscellaneous Tasks

- Install matrix target into pinned toolchain + add backfill workflow (#9)
- Bump fgumi to 0.3.0 (#10)

## [0.1.1] - 2026-05-21

### Documentation

- Clarify default sub-sort and lowercase tool name in README (#1)

### Features

- Default log level to warn; add -v/--verbose for info logs (#5)

### Miscellaneous Tasks

- Use GITHUB_TOKEN and trusted publishing in publish.yml (#2)
- Switch publish.yml to fg-labs-bot App pattern (#4)

### Testing

- Assert on log-format marker; polish --verbose doc (#6)
- Derive expected version from CARGO_PKG_VERSION (#7)

## [0.1.0]

### Features

- Initial release.
- `mako -i in.bam -o out.bam --order <order>` — sort SAM/BAM files by
  coordinate, queryname (lexicographic or natural), or
  template-coordinate.
- `mako --verify` — verify a file is correctly sorted without
  rewriting.
- Powered by the sort engine from
  [fgumi](https://github.com/fulcrumgenomics/fgumi):
  external merge-sort with parallel radix sort for in-memory chunks,
  spill-to-disk, and configurable temp-file compression.
