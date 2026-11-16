//! Where the drawing and the exports live, per platform.
//!
//! Linux follows the XDG base directories, macOS uses Application Support and
//! Downloads, Windows uses Roaming AppData and Pictures. Both locations can be
//! redirected with `LATTICE_DATA_DIR` and `LATTICE_EXPORT_DIR`, which is how
//! the tests keep their files out of the real home directory.

use std::path::PathBuf;

fn from_env(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(not(windows))]
fn home() -> Option<PathBuf> {
    from_env("HOME")
}

#[cfg(windows)]
fn data_base() -> PathBuf {
    from_env("APPDATA")
        .or_else(|| from_env("USERPROFILE").map(|home| home.join("AppData").join("Roaming")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Lattice")
}

#[cfg(target_os = "macos")]
fn data_base() -> PathBuf {
    home()
        .map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("Lattice")
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn data_base() -> PathBuf {
    from_env("XDG_DATA_HOME")
        .or_else(|| home().map(|home| home.join(".local").join("share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lattice")
}

#[cfg(windows)]
fn export_base() -> PathBuf {
    from_env("USERPROFILE")
        .map(|home| home.join("Pictures"))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(target_os = "macos")]
fn export_base() -> PathBuf {
    home()
        .map(|home| home.join("Downloads"))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn export_base() -> PathBuf {
    from_env("XDG_PICTURES_DIR")
        .or_else(|| home().map(|home| home.join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Directory that holds the saved drawing.
pub fn data_dir() -> PathBuf {
    from_env("LATTICE_DATA_DIR").unwrap_or_else(data_base)
}

/// Full path of the saved drawing. The directory is created if missing.
pub fn drawing_path() -> PathBuf {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir.join("drawing.json")
}

/// Directory exports are written to.
pub fn export_dir() -> PathBuf {
    from_env("LATTICE_EXPORT_DIR").unwrap_or_else(export_base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_win_over_platform_defaults() {
        // Safety: this test sets an environment variable for its own process.
        unsafe {
            std::env::set_var("LATTICE_DATA_DIR", "/tmp/lattice-data-probe");
            std::env::set_var("LATTICE_EXPORT_DIR", "/tmp/lattice-export-probe");
        }
        assert_eq!(data_dir(), PathBuf::from("/tmp/lattice-data-probe"));
        assert_eq!(export_dir(), PathBuf::from("/tmp/lattice-export-probe"));
        unsafe {
            std::env::remove_var("LATTICE_DATA_DIR");
            std::env::remove_var("LATTICE_EXPORT_DIR");
        }
    }
}
