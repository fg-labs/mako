# mako

> Fast SAM/BAM sorter.

[![crates.io](https://img.shields.io/crates/v/fg-mako.svg)](https://crates.io/crates/fg-mako)
[![CI](https://github.com/fg-labs/mako/actions/workflows/check.yml/badge.svg)](https://github.com/fg-labs/mako/actions/workflows/check.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`mako` is a focused, single-purpose command-line tool for sorting SAM and
BAM files. It sorts by coordinate, queryname (lexicographic or natural),
or template-coordinate, and can verify whether a file is already
correctly sorted without rewriting it.

## Highlights

- **External merge-sort** — handles BAM files larger than available RAM
  via spill-to-disk.
- **Parallel radix sort** for in-memory chunks.
- **Multiple sort orders:**
  - `coordinate` — standard genomic order (`tid → pos → strand`); use
    for IGV, variant calling, etc.
  - `queryname` — lexicographic read name (default sub-sort, fast).
  - `queryname::natural` — `samtools`-compatible natural numeric order.
  - `template-coordinate` — paired-end reads grouped by template
    position.
- **`--verify` mode** — check a file's sortedness in one streaming pass
  without writing output.
- **Drop-in for `samtools sort`** for the common cases.

## Install

From [crates.io](https://crates.io/crates/fg-mako) — note the
package name is `fg-mako` (the unprefixed `mako` is taken on
crates.io by an unrelated, abandoned crate), but the installed binary
is `mako`:

```sh
cargo install fg-mako
```

Or grab a pre-built binary from the
[GitHub releases page](https://github.com/fg-labs/mako/releases) for
Linux and macOS on x86_64 and aarch64.

## Usage

Sort by coordinate:

```sh
mako -i in.bam -o out.bam --order coordinate --threads 8
```

Sort by query name (samtools-compatible natural order):

```sh
mako -i in.bam -o out.bam --order queryname::natural
```

Verify sort order without rewriting:

```sh
mako -i sorted.bam --verify --order coordinate
```

Run `mako --help` for the full list of options including memory limits,
temp-directory placement, compression level, and verification mode.

## Sort orders

| `--order`              | Order                                  | Typical use                       |
| ---------------------- | -------------------------------------- | --------------------------------- |
| `coordinate`           | `tid → pos → strand`                   | IGV, variant calling, indexing    |
| `queryname`            | Lexicographic read name (default)      | Fast queryname sort               |
| `queryname::natural`   | Natural numeric read name              | `samtools`-compatible             |
| `template-coordinate`  | Paired reads grouped by template       | UMI grouping pipelines            |

## Performance

Mako inherits its sort engine from [fgumi](https://github.com/fulcrumgenomics/fgumi).
On a 30M-read WGS BAM, the engine sorts roughly 1.9× faster than
`samtools sort` for template-coordinate order. See the fgumi
documentation for detailed benchmarks and tuning guidance.

## Built on

`mako` is a focused packaging of the SAM/BAM sort engine from
[fgumi](https://github.com/fulcrumgenomics/fgumi). All sort logic lives
upstream; mako tracks fgumi releases and exists to provide a small,
stable, easily installed sort binary for users who don't need fgumi's
broader UMI tooling.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Issues that look like sort
engine bugs (correctness, performance) belong on the fgumi issue
tracker; mako-specific issues (CLI ergonomics, packaging, distribution)
belong here.

## License

MIT — see [LICENSE](LICENSE) and [THIRDPARTY.toml](THIRDPARTY.toml).
