use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Only run this logic if the genre-onnx feature is enabled
    if env::var("CARGO_FEATURE_GENRE_ONNX").is_ok() {
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
        let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

        // Construct the source path for the runtime library
        // Structure: libs/{os}-{arch}/
        // Example: libs/windows-x86_64/onnxruntime.dll
        let lib_dir_name = format!("{}-{}", target_os, target_arch);
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let source_dir = Path::new(&manifest_dir).join("libs").join(&lib_dir_name);
        // Also check assets/runtime which users might prefer
        // Structure check for: assets/runtime/win-x64/onnxruntime-win-x64-1.23.2/lib/
        // Since version might change, we might need a more robust search, but for now specific check:
        let assets_runtime_dir = Path::new(&manifest_dir).join("assets").join("runtime");

        let lib_name = if target_os == "windows" {
            "onnxruntime.dll"
        } else if target_os == "macos" {
            "libonnxruntime.dylib"
        } else {
            "libonnxruntime.so"
        };

        // Try simple libs/{os}-{arch} first
        let mut source_path = source_dir.join(lib_name);

        if !source_path.exists() {
            // Try to find it in assets/runtime/extracted_folder/lib
            // We know specifically for windows it might be in win-x64/onnxruntime-win-x64-1.23.2/lib/
            // Let's implement a simple search or specific path check based on what we just saw
            if target_os == "windows" && target_arch == "x86_64" {
                let specific = assets_runtime_dir
                    .join("win-x64")
                    .join("onnxruntime-win-x64-1.23.2")
                    .join("lib")
                    .join(lib_name);
                if specific.exists() {
                    source_path = specific;
                }
            }
        }

        // Determine output directory
        // This is a bit tricky in Rust build scripts, but usually OUT_DIR is for generated code.
        // For runtime DLLs, we want them next to the binary in target/release/ or target/debug/
        // However, build scripts can't easily access the final binary output dir.
        // A common workaround is to copy to the target directory relative to the manifest,
        // relying on the standard cargo layout.

        let profile = env::var("PROFILE").unwrap();
        let target_dir = Path::new(&manifest_dir).join("target").join(&profile);

        // Also try to copy to the deps folder which is sometimes where the binary runs from during tests
        let deps_dir = target_dir.join("deps");

        if source_path.exists() {
            println!(
                "cargo:warning=Found ONNX Runtime library at {:?}",
                source_path
            );

            // Attempt to create target dirs if they don't exist (cargo usually creates them though)
            let _ = fs::create_dir_all(&target_dir);
            let _ = fs::create_dir_all(&deps_dir);

            let dest_path = target_dir.join(lib_name);
            let dest_path_deps = deps_dir.join(lib_name);

            if let Err(e) = fs::copy(&source_path, &dest_path) {
                println!(
                    "cargo:warning=Failed to copy ONNX runtime to target dir: {}",
                    e
                );
            }
            if let Err(e) = fs::copy(&source_path, &dest_path_deps) {
                println!(
                    "cargo:warning=Failed to copy ONNX runtime to deps dir: {}",
                    e
                );
            }
        } else {
            println!("cargo:warning=ONNX Runtime library NOT found at {:?}. Please download it and place it there for the 'genre-onnx' feature to work.", source_path);
        }

        // Rerun if the library file changes or the libs directory changes
        println!("cargo:rerun-if-changed=libs");
    }
}
