//! T804 — `audit::query::ledger_snapshot_sha` integration tests.
//!
//! Acceptance:
//!  - byte-stable across two reads of the same file,
//!  - flipping a single byte changes the digest.

use std::io::{Seek, SeekFrom, Write};

use audit::query::ledger_snapshot_sha;

#[test]
fn t804_snapshot_sha_byte_stable_across_two_reads() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ledger.bin");

    // Write a deterministic 256 KiB blob — large enough to span multiple
    // chunked reads in the streaming hasher.
    let mut bytes = Vec::with_capacity(256 * 1024);
    for i in 0..(256 * 1024) {
        // u8 wraps every 256 bytes — good enough for a fixture.
        let b = u8::try_from(i % 256).expect("byte fits");
        bytes.push(b);
    }
    std::fs::write(&path, &bytes).expect("write fixture");

    let h1 = ledger_snapshot_sha(&path).expect("first hash");
    let h2 = ledger_snapshot_sha(&path).expect("second hash");
    assert_eq!(
        h1, h2,
        "two reads of the same file must produce the same SHA"
    );
}

#[test]
fn t804_snapshot_sha_flipping_one_byte_changes_digest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ledger.bin");

    // Write a 4 KiB fixture.
    let bytes = vec![0xABu8; 4096];
    std::fs::write(&path, &bytes).expect("write fixture");
    let h_before = ledger_snapshot_sha(&path).expect("hash before");

    // Flip one byte at offset 1024.
    {
        let mut f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("reopen rw");
        f.seek(SeekFrom::Start(1024)).expect("seek");
        f.write_all(&[0x00u8]).expect("flip byte");
    }
    let h_after = ledger_snapshot_sha(&path).expect("hash after");
    assert_ne!(
        h_before, h_after,
        "flipping one byte must change the digest"
    );
}

#[test]
fn t804_snapshot_sha_known_vector_empty_file() {
    // SHA-256 of the empty input is a well-known constant.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty.bin");
    std::fs::write(&path, b"").expect("write empty");
    let h = ledger_snapshot_sha(&path).expect("hash empty");
    let expected_hex = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let actual_hex: String = h.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(actual_hex, expected_hex, "SHA-256 of empty input");
}
