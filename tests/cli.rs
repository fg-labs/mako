//! End-to-end CLI tests for `mako`.
//!
//! These tests exercise the binary as a black box: they build small BAM
//! files programmatically with `noodles`, invoke the compiled `mako`
//! binary, and inspect its exit status and outputs. They prove the
//! wiring (clap flatten, command_line plumbing, exit codes) — not the
//! correctness of fgumi's underlying sort, which is exercised upstream.
//!
//! The `cat file | mako` stdin tests spawn `cat` children whose stdout is
//! consumed as mako's stdin; they are reaped when the pipe is drained, so
//! the never-`wait`ed `Child` is intentional.
#![allow(clippy::zombie_processes)]

use std::fs::File;
use std::num::NonZeroUsize;
use std::path::Path;
use std::process::{Command, Stdio};

use assert_cmd::cargo::CommandCargoExt;
use noodles::bam;
use noodles::sam::{
    self,
    alignment::{RecordBuf, io::Write as AlignmentWrite, record::Flags},
    header::record::value::{Map, map::ReferenceSequence},
};
use tempfile::TempDir;

/// One reference, length 100_000. Sufficient to fit any toy test position.
fn build_header() -> sam::Header {
    sam::Header::builder()
        .add_reference_sequence(
            "chr1",
            Map::<ReferenceSequence>::new(NonZeroUsize::new(100_000).unwrap()),
        )
        .build()
}

/// Build a single mapped, unpaired record with the given name and 1-based
/// alignment start position.
fn record(name: &str, position: usize) -> RecordBuf {
    let mut rec = RecordBuf::builder()
        .set_name(name.as_bytes())
        .set_flags(Flags::default()) // mapped, unpaired
        .set_reference_sequence_id(0)
        .set_alignment_start(noodles::core::Position::new(position).unwrap())
        .build();
    // Default flags includes UNMAPPED (0x4); clear it so we have a mapped
    // record (so coordinate sort actually orders by tid → pos).
    *rec.flags_mut() = Flags::empty();
    rec
}

/// Write a BAM containing the given records under the standard header.
fn write_bam(path: &Path, records: &[RecordBuf]) {
    let header = build_header();
    let mut writer = bam::io::Writer::new(File::create(path).unwrap());
    writer.write_header(&header).unwrap();
    for rec in records {
        writer.write_alignment_record(&header, rec).unwrap();
    }
    writer.try_finish().unwrap();
}

/// Read a BAM and return (header, records-as-RecordBuf).
fn read_bam(path: &Path) -> (sam::Header, Vec<RecordBuf>) {
    let mut reader = bam::io::reader::Builder.build_from_path(path).unwrap();
    let header = reader.read_header().unwrap();
    let records: Vec<RecordBuf> = reader.record_bufs(&header).map(|r| r.unwrap()).collect();
    (header, records)
}

fn mako() -> Command {
    Command::cargo_bin("mako").unwrap()
}

// ---------------------------------------------------------------------------
// Test 1: --help
// ---------------------------------------------------------------------------

#[test]
fn help_prints_flat_sort_options() {
    let out = mako().arg("--help").output().unwrap();
    assert!(out.status.success(), "mako --help should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Proves flatten worked: the inner Sort flags surface at the top level.
    assert!(stdout.contains("--input"), "missing --input in help: {stdout}");
    assert!(stdout.contains("--output"), "missing --output in help: {stdout}");
    assert!(stdout.contains("--order"), "missing --order in help: {stdout}");
    // Proves the parent `about` won (not fgumi's [ALIGNMENT] banner).
    assert!(stdout.contains("Fast SAM/BAM sorter"), "expected mako tagline in help: {stdout}");
}

// ---------------------------------------------------------------------------
// Test 2: --version
// ---------------------------------------------------------------------------

#[test]
fn version_includes_fgumi_attribution() {
    let out = mako().arg("--version").output().unwrap();
    assert!(out.status.success(), "mako --version should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let expected_prefix = format!("mako {}", env!("CARGO_PKG_VERSION"));
    assert!(stdout.starts_with(&expected_prefix), "unexpected version line: {stdout}");
    assert!(stdout.contains("powered by fgumi"), "missing fgumi attribution: {stdout}");
}

// ---------------------------------------------------------------------------
// Test 3: end-to-end coordinate sort
// ---------------------------------------------------------------------------

#[test]
fn coordinate_sort_orders_records_by_position() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("in.bam");
    let output = tmp.path().join("out.bam");

    // Out-of-order positions.
    write_bam(&input, &[record("a", 500), record("b", 100), record("c", 300)]);

    let status = mako()
        .args(["-i", input.to_str().unwrap()])
        .args(["-o", output.to_str().unwrap()])
        .args(["--order", "coordinate"])
        .status()
        .unwrap();
    assert!(status.success(), "mako exited non-zero on coordinate sort");

    let (header, records) = read_bam(&output);
    let positions: Vec<usize> =
        records.iter().map(|r| r.alignment_start().unwrap().get()).collect();
    assert_eq!(positions, vec![100, 300, 500], "records not coordinate-sorted");

    // Header SO tag should reflect the new sort order.
    let so = header
        .header()
        .and_then(|h| h.other_fields().get(b"SO"))
        .map(|v| v.to_string())
        .unwrap_or_default();
    assert_eq!(so, "coordinate", "expected SO:coordinate, got {so:?}");
}

// ---------------------------------------------------------------------------
// Test 4: end-to-end queryname sort
// ---------------------------------------------------------------------------

#[test]
fn queryname_sort_orders_records_lexicographically() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("in.bam");
    let output = tmp.path().join("out.bam");

    write_bam(&input, &[record("zebra", 100), record("apple", 200), record("mango", 300)]);

    let status = mako()
        .args(["-i", input.to_str().unwrap()])
        .args(["-o", output.to_str().unwrap()])
        .args(["--order", "queryname"])
        .status()
        .unwrap();
    assert!(status.success(), "mako exited non-zero on queryname sort");

    let (header, records) = read_bam(&output);
    let names: Vec<String> =
        records.iter().map(|r| String::from_utf8_lossy(r.name().unwrap()).into_owned()).collect();
    assert_eq!(names, vec!["apple", "mango", "zebra"], "records not queryname-sorted");

    let so = header
        .header()
        .and_then(|h| h.other_fields().get(b"SO"))
        .map(|v| v.to_string())
        .unwrap_or_default();
    assert_eq!(so, "queryname", "expected SO:queryname, got {so:?}");
}

// ---------------------------------------------------------------------------
// Test 5: --verify on a sorted file exits 0
// ---------------------------------------------------------------------------

#[test]
fn verify_passes_on_sorted_input() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("sorted.bam");
    let sorted = tmp.path().join("via-mako-sorted.bam");

    // Generate an in-order coord-sorted BAM by sorting it with mako first.
    write_bam(&input, &[record("a", 500), record("b", 100), record("c", 300)]);
    let status = mako()
        .args(["-i", input.to_str().unwrap()])
        .args(["-o", sorted.to_str().unwrap()])
        .args(["--order", "coordinate"])
        .status()
        .unwrap();
    assert!(status.success());

    // Verify the sorted file claims to be sorted.
    let status = mako()
        .args(["-i", sorted.to_str().unwrap()])
        .args(["--verify"])
        .args(["--order", "coordinate"])
        .status()
        .unwrap();
    assert!(status.success(), "verify should pass on sorted input");
}

// ---------------------------------------------------------------------------
// Test 6: --verify on an unsorted file exits non-zero
// ---------------------------------------------------------------------------

#[test]
fn verify_fails_on_unsorted_input() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("unsorted.bam");
    write_bam(&input, &[record("a", 500), record("b", 100), record("c", 300)]);

    let status = mako()
        .args(["-i", input.to_str().unwrap()])
        .args(["--verify"])
        .args(["--order", "coordinate"])
        .status()
        .unwrap();
    assert!(!status.success(), "verify should fail on unsorted input");
}

// ---------------------------------------------------------------------------
// Test 7: default logging is quiet; -v opts in to info-level logs
// ---------------------------------------------------------------------------

#[test]
fn verbose_flag_enables_info_logs() {
    // Asserts on env_logger's default level field (` INFO `) so the test
    // stays valid if fgumi renames or removes any specific info-level
    // log line. env_logger's default format is
    // `[<ts> <LEVEL> <module>] <msg>` with the level padded to 5 chars.
    const INFO_MARKER: &str = " INFO ";

    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("in.bam");
    let output_quiet = tmp.path().join("out-quiet.bam");
    let output_verbose = tmp.path().join("out-verbose.bam");
    write_bam(&input, &[record("a", 500), record("b", 100)]);

    let out = mako()
        .env_remove("RUST_LOG")
        .args(["-i", input.to_str().unwrap()])
        .args(["-o", output_quiet.to_str().unwrap()])
        .args(["--order", "coordinate"])
        .output()
        .unwrap();
    assert!(out.status.success(), "default sort should succeed");
    let stderr_quiet = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr_quiet.contains(INFO_MARKER),
        "default run should suppress info logs, got: {stderr_quiet}"
    );

    let out = mako()
        .env_remove("RUST_LOG")
        .args(["-i", input.to_str().unwrap()])
        .args(["-o", output_verbose.to_str().unwrap()])
        .args(["--order", "coordinate"])
        .arg("-v")
        .output()
        .unwrap();
    assert!(out.status.success(), "-v sort should succeed");
    let stderr_verbose = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr_verbose.contains(INFO_MARKER),
        "-v should surface info logs, got: {stderr_verbose}"
    );
}

// ---------------------------------------------------------------------------
// Test 8: nonexistent input → non-zero exit with diagnostic
// ---------------------------------------------------------------------------

#[test]
fn nonexistent_input_exits_nonzero() {
    let tmp = TempDir::new().unwrap();
    let bogus = tmp.path().join("does-not-exist.bam");
    let output = tmp.path().join("out.bam");

    let out = mako()
        .args(["-i", bogus.to_str().unwrap()])
        .args(["-o", output.to_str().unwrap()])
        .args(["--order", "coordinate"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "expected non-zero exit on missing input");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.is_empty(), "expected a diagnostic on stderr");
}

// ---------------------------------------------------------------------------
// Test 9: sort reads a BAM piped on stdin (parity with file input)
// ---------------------------------------------------------------------------

/// Pipe `input` to `mako` via `cat` (so stdin is a non-seekable stream, like a
/// real `cat foo.bam | mako` pipe) and coordinate-sort it to `output`, reading
/// from the given `input_arg` (`-` or `/dev/stdin`). Asserts a clean exit.
fn sort_from_piped_stdin(input: &Path, output: &Path, input_arg: &str) {
    let cat = Command::new("cat")
        .arg(input.to_str().unwrap())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn cat");
    let status = mako()
        .args(["-i", input_arg])
        .args(["-o", output.to_str().unwrap()])
        .args(["--order", "coordinate"])
        .stdin(cat.stdout.expect("cat stdout"))
        .status()
        .unwrap();
    assert!(status.success(), "mako coordinate sort from stdin ({input_arg}) exited non-zero");
}

/// Sorting a BAM streamed on stdin must produce the same records as sorting the
/// same BAM from a file. Mirrors fgumi's `test_sort_reads_stdin_once`: it guards
/// against regressions where stdin is dropped, double-opened, or read twice.
///
/// Both `-` and `/dev/stdin` are exercised: `/dev/stdin` is a real path
/// (`Path::exists` is true), so a stdin gate keyed only on the literal `-`
/// would mishandle it.
#[test]
fn sort_from_stdin_matches_file_input() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("in.bam");
    write_bam(&input, &[record("a", 500), record("b", 100), record("c", 300)]);

    // Baseline: sort from a file.
    let out_file = tmp.path().join("out-file.bam");
    let status = mako()
        .args(["-i", input.to_str().unwrap()])
        .args(["-o", out_file.to_str().unwrap()])
        .args(["--order", "coordinate"])
        .status()
        .unwrap();
    assert!(status.success(), "baseline file sort exited non-zero");

    let (_, expected) = read_bam(&out_file);
    // Guard against a vacuous pass: the baseline must actually emit records,
    // otherwise the parity comparison below would be trivially true even if
    // stdin were dropped entirely.
    assert_eq!(expected.len(), 3, "baseline sort should emit all 3 records");

    for input_arg in ["-", "/dev/stdin"] {
        let out_pipe = tmp.path().join("out-pipe.bam");
        sort_from_piped_stdin(&input, &out_pipe, input_arg);
        let (_, actual) = read_bam(&out_pipe);
        assert_eq!(
            actual, expected,
            "records from stdin ({input_arg}) differ from file-input sort"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 10: --verify rejects a non-seekable stdin stream with a clear message
// ---------------------------------------------------------------------------

/// `--verify` re-reads its input (header probe + a fresh record re-scan), which
/// a non-seekable stdin stream can't satisfy, so it must be rejected up front
/// with a message that mentions stdin — not fail cryptically deep in a re-open.
/// Mirrors fgumi's `test_sort_verify_rejects_stdin_with_clear_message`; covers
/// both `-` and `/dev/stdin` since the latter is a real, existing path.
#[test]
fn verify_rejects_stdin_input() {
    for input_arg in ["-", "/dev/stdin"] {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("in.bam");
        write_bam(&input, &[record("a", 500), record("b", 100), record("c", 300)]);

        let cat = Command::new("cat")
            .arg(input.to_str().unwrap())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn cat");
        let out = mako()
            .args(["-i", input_arg])
            .args(["--verify"])
            .args(["--order", "coordinate"])
            .stdin(cat.stdout.expect("cat stdout"))
            .output()
            .unwrap();

        assert!(
            !out.status.success(),
            "--verify from stdin ({input_arg}) must be rejected, not succeed"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("stdin"),
            "--verify stdin ({input_arg}) rejection should mention stdin; stderr: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 11: --compression-level passthrough (level 0 = uncompressed, for pipes)
// ---------------------------------------------------------------------------

/// `--compression-level` flows through the flattened `Sort` to the spill-merge
/// output BGZF writer. Level 0 writes uncompressed (stored) BGZF — a valid BAM
/// with no DEFLATE — which is the fast path when piping the sort into another
/// process: skipping output compression measurably speeds the sort up, and the
/// downstream tool re-reads (or recompresses) anyway.
///
/// This locks in two things a future fgumi bump must not silently regress:
/// (1) the flag actually reaches the writer — level-0 output is materially
/// larger than level-1 for the same compressible records; and (2) the
/// compression level never alters the records — both levels decode to the same
/// coordinate-sorted set.
///
/// The input is forced to spill (tiny `--max-memory`), because the
/// piped/uncompressed use case is large inputs, which always spill and merge —
/// and that merge writer is the one that honors level 0.
#[test]
fn compression_level_passthrough_level0_is_uncompressed() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("in.bam");

    // Many records with repetitive content and unique, descending positions:
    // unique keys make the coordinate sort fully deterministic (no
    // timing-dependent tie-break), and the repetition makes DEFLATE (level 1)
    // compress markedly better than stored (level 0), so the size gap is
    // unambiguous. Written descending so the sort has real reordering to do.
    let mut records: Vec<RecordBuf> = (0..8000).map(|i| record("read", 1 + i * 7)).collect();
    records.reverse();
    write_bam(&input, &records);

    let out0 = tmp.path().join("out0.bam");
    let out1 = tmp.path().join("out1.bam");
    for (out, level) in [(&out0, "0"), (&out1, "1")] {
        let status = mako()
            .args(["-i", input.to_str().unwrap()])
            .args(["-o", out.to_str().unwrap()])
            .args(["--order", "coordinate"])
            // Tiny total memory budget forces multiple spills → the k-way merge
            // writer, which is the output path the piped/uncompressed case uses.
            .args(["--max-memory", "64K"])
            .args(["--memory-per-thread", "false"])
            .args(["--compression-level", level])
            .status()
            .unwrap();
        assert!(status.success(), "mako exited non-zero at --compression-level {level}");
    }

    // Both levels must decode to the same, coordinate-sorted records: the output
    // compression level must never change the records themselves.
    let (_, recs0) = read_bam(&out0);
    let (_, recs1) = read_bam(&out1);
    assert_eq!(recs0, recs1, "level 0 and level 1 must decode to identical records");
    assert_eq!(recs0.len(), records.len(), "record count must be preserved");
    let positions: Vec<usize> = recs0.iter().map(|r| r.alignment_start().unwrap().get()).collect();
    let mut sorted = positions.clone();
    sorted.sort_unstable();
    assert_eq!(positions, sorted, "records must be coordinate-sorted at level 0");

    // The observable signal that the flag reached the BGZF writer: level-0
    // (stored) output is materially larger than level-1 (DEFLATE) for this
    // compressible data. Equal sizes would mean the level never took effect.
    let size0 = std::fs::metadata(&out0).unwrap().len();
    let size1 = std::fs::metadata(&out1).unwrap().len();
    assert!(
        size0 > size1,
        "uncompressed (level 0, {size0} B) should exceed compressed (level 1, {size1} B)"
    );
}
