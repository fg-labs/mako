#![deny(unsafe_code)]

use anyhow::Result;
use clap::Parser;
use env_logger::Env;
use fgumi_lib::commands::{command::Command, sort::Sort};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Fast SAM/BAM sorter.
///
/// `mako` is a focused, single-purpose sort utility for SAM/BAM files,
/// powered by the sort engine from
/// [fgumi](https://github.com/fulcrumgenomics/fgumi).
///
/// Supports coordinate, queryname (lexicographic and natural), and
/// template-coordinate sort orders, as well as a `--verify` mode that
/// checks sortedness without rewriting.
#[derive(Parser)]
#[command(
    name = "mako",
    version = mako_long_version(),
    about = "Fast SAM/BAM sorter.",
    long_about = None,
)]
struct Cli {
    #[command(flatten)]
    sort: Sort,
}

fn mako_long_version() -> &'static str {
    // Includes the mako version, the git rev mako was built from, and the
    // resolved fgumi version. Built once at compile time via the `built`
    // crate.
    static LONG_VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    LONG_VERSION.get_or_init(|| {
        let rev = built_info::GIT_COMMIT_HASH_SHORT.unwrap_or("unknown");
        let fgumi_ver = built_info::DEPENDENCIES
            .iter()
            .find_map(|(name, version)| (*name == "fgumi").then_some(*version))
            .unwrap_or("unknown");
        format!("{} (rev {rev})\npowered by fgumi {fgumi_ver}", built_info::PKG_VERSION)
    })
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Capture the original invocation for the @PG header record before clap
    // consumes it.
    let command_line = std::env::args().collect::<Vec<_>>().join(" ");

    let cli = Cli::parse();
    cli.sort.execute(&command_line)
}
