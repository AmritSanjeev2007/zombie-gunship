//! Locating files in the `assets/` folder.

use std::path::{Path, PathBuf};

/// Resolves a path relative to the `assets/` folder.
///
/// Looks next to the executable first (for shipped builds), then in the
/// current working directory, then in the crate root (for `cargo run`).
pub fn path(relative: impl AsRef<Path>) -> PathBuf {
    let relative = relative.as_ref();

    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        roots.push(dir.join("assets"));
    }
    roots.push(PathBuf::from("assets"));
    roots.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("assets"));

    roots
        .iter()
        .map(|root| root.join(relative))
        .find(|candidate| candidate.exists())
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(relative)
        })
}
