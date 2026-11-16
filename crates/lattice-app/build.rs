//! Links `libxkbcommon-x11` on Linux only.
//!
//! The runtime package ships `libxkbcommon-x11.so.0` without the plain `.so`
//! symlink the linker wants. On Linux this looks for a usable file, and when
//! only the versioned one exists it creates a symlink in `OUT_DIR` and puts
//! that directory on the link search path. Nothing is written into the source
//! tree. Windows and macOS builds add nothing.

use std::path::{Path, PathBuf};

const DIRS: [&str; 7] = [
    "/usr/lib/x86_64-linux-gnu",
    "/usr/lib/aarch64-linux-gnu",
    "/usr/lib64",
    "/usr/lib",
    "/lib/x86_64-linux-gnu",
    "/lib/aarch64-linux-gnu",
    "/usr/local/lib",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        return;
    }

    for dir in DIRS {
        if Path::new(dir).join("libxkbcommon-x11.so").exists() {
            println!("cargo:rustc-link-search=native={dir}");
            return;
        }
    }

    for dir in DIRS {
        let versioned = Path::new(dir).join("libxkbcommon-x11.so.0");
        if !versioned.exists() {
            continue;
        }
        let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
        let link = out.join("libxkbcommon-x11.so");
        let _ = std::fs::remove_file(&link);
        #[cfg(unix)]
        if std::os::unix::fs::symlink(&versioned, &link).is_ok() {
            println!("cargo:rustc-link-search=native={}", out.display());
            return;
        }
    }

    println!("cargo:warning=libxkbcommon-x11 not found; install libxkbcommon-x11-0 to build");
}
