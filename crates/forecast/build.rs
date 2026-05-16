//! Build script for `crates/forecast`.
//!
//! ## Checksum gate (T-M2-4)
//!
//! Asserts that `crates/forecast/assets/kronos-base.onnx` has the SHA-256
//! hash pinned in `crates/forecast/assets/kronos-base.onnx.sha256`.
//!
//! ### Behaviour
//!
//! - If `kronos-base.onnx` does NOT exist: prints a `cargo:warning` and
//!   skips the gate. This allows `cargo build` to succeed on developer
//!   machines that haven't run the LFS bootstrap yet (or in CI with
//!   `--no-lfs`). The [`crate::kronos::KronosForecaster`] will return
//!   `ForecastError::Inference` at runtime (stub until M3 anyway).
//!
//! - If `kronos-base.onnx` EXISTS but the `.sha256` file says `PENDING`:
//!   skips the gate (LFS not yet populated).
//!
//! - If `kronos-base.onnx` EXISTS and `.sha256` contains a real hash:
//!   asserts SHA-256 matches. `cargo build -p forecast` fails if the
//!   checkpoint mutates.
//!
//! ### Re-run triggers
//!
//! `cargo:rerun-if-changed=assets/kronos-base.onnx` — the gate reruns
//! whenever the ONNX file changes (e.g. after `git lfs pull`).

use std::io::Read;
use std::path::PathBuf;

fn main() {
    // Tell Cargo to rerun this script if the ONNX file changes.
    println!("cargo:rerun-if-changed=assets/kronos-base.onnx");
    println!("cargo:rerun-if-changed=assets/kronos-base.onnx.sha256");

    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"),
    );
    let onnx_path = manifest_dir.join("assets").join("kronos-base.onnx");
    let sha256_path = manifest_dir
        .join("assets")
        .join("kronos-base.onnx.sha256");

    // Read the pinned hash from the .sha256 file.
    let pinned_hash = match std::fs::read_to_string(&sha256_path) {
        Ok(contents) => {
            // Strip comments and whitespace.
            contents
                .lines()
                .find(|l| !l.starts_with('#') && !l.trim().is_empty())
                .map(|l| l.trim().to_string())
                .unwrap_or_default()
        }
        Err(e) => {
            println!(
                "cargo:warning=forecast/build.rs: cannot read {}: {e}",
                sha256_path.display()
            );
            return;
        }
    };

    // If the pinned hash is PENDING, skip the gate.
    if pinned_hash == "PENDING" || pinned_hash.is_empty() {
        println!(
            "cargo:warning=forecast/build.rs: ONNX checksum gate SKIPPED — \
             kronos-base.onnx.sha256 says PENDING. Run LFS bootstrap to activate."
        );
        return;
    }

    // If the ONNX file doesn't exist, skip the gate with a warning.
    if !onnx_path.exists() {
        println!(
            "cargo:warning=forecast/build.rs: ONNX checksum gate SKIPPED — \
             {} not found. Run `git lfs pull` or the LFS bootstrap.",
            onnx_path.display()
        );
        return;
    }

    // File exists and hash is pinned — run the checksum gate.
    let actual_hash = sha256_file(&onnx_path);
    if actual_hash != pinned_hash {
        // Use panic so cargo surfaces it as a build error with a clear message.
        panic!(
            "forecast/build.rs: ONNX checksum mismatch!\n\
             file:    {}\n\
             expected: {}\n\
             actual:   {}\n\
             The checkpoint mutated. Update the .sha256 file after verifying the new checkpoint.",
            onnx_path.display(),
            pinned_hash,
            actual_hash
        );
    }

    println!(
        "cargo:warning=forecast/build.rs: ONNX checksum OK ({}).",
        &actual_hash[..16]
    );
}

/// Compute SHA-256 hex of a file.
fn sha256_file(path: &std::path::Path) -> String {
    use std::io::BufReader;

    // Manual SHA-256 using the standard library's `sha2` crate is not
    // available in build scripts without adding it as a build-dependency.
    // We use a portable pure-Rust implementation via the `sha2` workspace
    // dependency declared under `[build-dependencies]` in Cargo.toml.
    //
    // For the build script we use a simpler approach: invoke `shasum`/`sha256sum`
    // as a subprocess only if the sha2 crate is not available.
    //
    // Since `sha2` IS declared as a build-dependency below, we can use it here.
    use sha2::{Digest, Sha256};

    let file = std::fs::File::open(path)
        .unwrap_or_else(|e| panic!("build.rs: cannot open {}: {e}", path.display()));
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 65536];
    loop {
        let n = reader.read(&mut buf).expect("read");
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    hex_lower(&digest)
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}
