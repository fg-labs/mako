//! End-to-end CLI tests for `mako`.
//!
//! These tests exercise the binary as a black box: they build small BAM
//! files programmatically with `noodles`, invoke the compiled `mako`
//! binary, and inspect its exit status and outputs. They prove the
//! wiring (clap flatten, command_line plumbing, exit codes) — not the
//! correctness of fgumi's underlying sort, which is exercised upstream.

use std::fs::File;
use std::num::NonZeroUsize;
use std::path::Path;
use std::process::Command;

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
    assert!(stdout.starts_with("mako 0.1.0"), "unexpected version line: {stdout}");
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
// Test 7: nonexistent input → non-zero exit with diagnostic
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
