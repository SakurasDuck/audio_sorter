use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    // Source path: ../out_lib/fpcalc.exe relative to the crate root
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let source_path = Path::new(&manifest_dir).join("../out_lib/fpcalc.exe");

    println!("cargo:rerun-if-changed={}", source_path.display());

    if !source_path.exists() {
        println!(
            "cargo:warning=fpcalc.exe not found at {}",
            source_path.display()
        );
        return;
    }

    // Destination: We want to put it next to the binary in target/debug/ or target/release/
    // OUT_DIR is usually target/debug/build/crate-hash/out
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Attempt to find the profile directory (debug or release)
    // Traverse up until we see "build" directory, then go one level up from that?
    // Structure: target / debug / build / crate / out
    // So target_dir is out_dir.parent().parent().parent() ?
    // Let's rely on a heuristic to find the "deps" sibling, since the binary is usually in the parent of "deps".

    let mut target_dir = out_dir.clone();
    let mut found = false;
    for _ in 0..5 {
        if let Some(parent) = target_dir.parent() {
            target_dir = parent.to_path_buf();
            // Check if "deps" exists in this directory
            if target_dir.join("deps").exists() {
                found = true;
                break;
            }
        } else {
            break;
        }
    }

    if !found {
        println!("cargo:warning=Could not determine target directory, copying to OUT_DIR only.");
        // Fallback: copy to OUT_DIR (won't help run, but better than nothing)
        let _ = fs::copy(&source_path, out_dir.join("fpcalc.exe"));
        return;
    }

    // Copy to the target directory (where the exe is)
    let dest_path = target_dir.join("fpcalc.exe");
    if let Err(e) = fs::copy(&source_path, &dest_path) {
        println!("cargo:warning=Failed to copy fpcalc.exe: {}", e);
    }
}
