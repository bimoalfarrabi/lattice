//! Build time glue for the two things a desktop binary needs that Cargo does
//! not do by itself.
//!
//! On Linux it links `libxkbcommon-x11`, whose runtime package ships
//! `libxkbcommon-x11.so.0` without the plain `.so` symlink the linker wants. It
//! looks for a usable file, and when only the versioned one exists it creates a
//! symlink in `OUT_DIR` and puts that directory on the link search path.
//! Nothing is written into the source tree.
//!
//! On Windows it embeds the icon and a version resource, so the executable has
//! its own icon and file properties instead of borrowing the installer's.

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

    #[cfg(windows)]
    embed_windows_resources();

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

/// Adds the application icon and the file properties Windows shows in Explorer.
/// The version numbers come from `CARGO_PKG_VERSION`.
#[cfg(windows)]
fn embed_windows_resources() {
    let icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/icons/lattice.ico");
    println!("cargo:rerun-if-changed={}", icon.display());

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon(icon.to_str().expect("icon path is valid UTF-8"));
    resource.set("FileDescription", "Lattice, a grid logo maker");
    resource.set("ProductName", "Lattice");
    resource.set("LegalCopyright", "MIT licensed");
    resource.set("OriginalFilename", "Lattice.exe");

    if let Err(error) = resource.compile() {
        println!("cargo:warning=could not embed the Windows resource: {error}");
    }
}
