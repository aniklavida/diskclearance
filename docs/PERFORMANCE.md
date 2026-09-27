# Performance evidence

## Provenance and scope

These are recorded measurements, not estimates. Every measurement in this document was taken on the same machine:

- **Hardware:** Mac mini, Apple M4, 16 GB memory (`sysctl -n machdep.cpu.brand_string` returned `Apple M4`; `system_profiler SPHardwareDataType` reported `Mac mini` and `16 GB`).
- **Operating system:** macOS 26.3, build 25D125 (`sw_vers`).
- **Architecture scope:** Apple Silicon only. DiskClearance v1.0 does not support Intel Macs, so there is no Intel measurement and none is implied.
- **Build:** release-mode Rust measurement binary, `measure_performance`, built with the repository's Rust 1.88 toolchain.

The budgets below are targets until replaced by the measured result in the same row. The measured results are one run on the named machine; they are not a claim about other Macs.

## Scan budgets

The fixtures contain exactly 10,000 and 1,000,000 filesystem entries, distributed across at most 1,000 directories. The scan uses the streaming `collect_entries: false` path.

| Budget target                                                                                                      | Measured result on Apple M4, macOS 26.3 build 25D125                                                                                                                           |
| ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Peak resident memory must stay within 32 MiB of the 10,000-entry scan when the fixture grows to 1,000,000 entries. | 9,109,504 bytes at 10,000 entries; 9,076,736 bytes at 1,000,000 entries; change: -32,768 bytes. The traversal record high-water mark was 1 in both runs.                       |
| Time to first verified result must be at most 10 ms.                                                               | 0.146750 ms at 10,000 entries; 1.386458 ms at 1,000,000 entries.                                                                                                               |
| Cancellation request-to-return latency must be at most 50 ms.                                                      | 0.148041 ms after cancellation was requested once 100,000 entries had been verified in a 1,000,000-entry fixture; the scan stopped at 100,001 entries and reported incomplete. |
| Progress-event emission under the 1,000,000-entry load must be at most 21 events/second.                           | 427 progress events over 21.305769 seconds: 20.041520 events/second. The 10,000-entry comparison emitted 4 events over 0.129506 seconds: 30.886490 events/second.              |

The 1,000,000-entry scan completed in 21.305769 seconds on Apple M4, macOS 26.3 build 25D125. The first result is emitted after entry metadata has been read and the entry has passed the traversal path; the cancellation result retains the partial count and does not report completion.

The CI regression is `scan::traversal::tests::test_scanning_1m_entries_keeps_peak_memory_flat_relative_to_10k_entries`. It runs as a normal macOS Rust test, rather than an ignored benchmark, and asserts both the streaming record count and the resident-memory comparison. The cancellation regression is `scan::traversal::tests::test_cancellation_returns_promptly_from_a_100k_entry_fixture`.

## Duplicate hashing

The duplicate probe used two identical 512 MiB files. It ran staged duplicate detection first, then a separate direct streaming-hash pass over the same 1,073,741,824 bytes.

| Budget target                                                             | Measured result on Apple M4, macOS 26.3 build 25D125                                                                            |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Full-content hashing must not use a read buffer larger than 65,536 bytes. | 65,536 bytes maximum observed; 1,073,741,824 bytes streamed in 2.065080 seconds.                                                |
| The duplicate probe must remain memory-bounded while hashing large media. | 9,863,168 bytes peak resident; staged detection completed in 2.322515 seconds and computed 2 full hashes for 1 confirmed group. |

The direct streaming pass is the memory evidence for `compute_full_hash_streaming`; the staged pass verifies that full hashing runs only for the two size-and-fingerprint survivors.

## Awkward cases

- **Large tree:** The release scan on Apple M4, macOS 26.3 build 25D125 visited all 1,000,000 entries in 21.305769 seconds and completed with the streaming memory comparison above.
- **Low disk space:** On Apple M4, macOS 26.3 build 25D125, a 16 MiB APFS disk image was mounted for a disposable write probe. A 64 MiB write request wrote 14,680,064 bytes and then returned `No space left on device` with exit status 1. The image was detached after the probe. This is a real filesystem exhaustion result, not an estimated application budget.
- **Permission-denied scope:** The existing Unix fixture test on Apple M4, macOS 26.3 build 25D125 used a mode-`0o000` restricted directory inside a disposable tree. It passed with one permission-denied scope, one coverage warning, a partial result, and the readable file retained.
- **Interrupted scan:** The release cancellation probe on Apple M4, macOS 26.3 build 25D125 stopped at 100,001 of 1,000,000 fixture entries after the request and reported `cancel_complete=false`; verified partial results were retained.
- **Corrupt database:** The storage fixture test on Apple M4, macOS 26.3 build 25D125 wrote malformed database bytes, passed, preserved the original content under a `.corrupt.<timestamp>` name, created a fresh database, returned `ok` from `PRAGMA quick_check(1)`, and reached schema version 4.

The repository's portable CI runs the permission, interrupted-scan, corrupt-database, cancellation, and large-tree tests. The low-disk APFS image probe is a macOS filesystem check that is not portable to the Windows CI runner.

## Reproduction

The measurement binary is `src-tauri/src/bin/measure_performance.rs` and is run with:

```text
cargo run --release --manifest-path src-tauri/Cargo.toml --bin measure_performance -- scan 1000000
cargo run --release --manifest-path src-tauri/Cargo.toml --bin measure_performance -- cancel 1000000 100000
cargo run --release --manifest-path src-tauri/Cargo.toml --bin measure_performance -- duplicate 536870912
```

The committed figures above were measured on Apple M4 with macOS 26.3 build 25D125. Re-running on another machine must produce and record that machine's own values rather than reusing these numbers.
