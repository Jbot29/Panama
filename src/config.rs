// Portable data layout: every path is derived from a single base directory
// resolved once at startup. The base is found by walking up from the executable
// until we hit the folder that carries `tutors/`.
//
//   - Distributed build: the binary sits beside `tutors/`, `media/`, `srs.db`,
//     so the base is found on the first step. Copy the folder anywhere and it
//     keeps working — a portable app, no installer.
//   - Dev build (`cargo run`): the exe lives in `target/release/`, so we walk
//     up to the repo root where `tutors/` lives.
//
// No machine-specific absolute paths. To relocate data later we only need to
// change how the base resolves (e.g. an env var or OS data dir) — every other
// path follows automatically.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn resolve_base_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        while let Some(d) = dir {
            if d.join("tutors").is_dir() {
                return d;
            }
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    // Fallback: current working directory.
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// The resolved base directory, computed once and cached.
pub fn base_dir() -> &'static Path {
    static BASE: OnceLock<PathBuf> = OnceLock::new();
    BASE.get_or_init(resolve_base_dir)
}

pub fn db_path() -> PathBuf {
    base_dir().join("srs.db")
}

pub fn media_images() -> PathBuf {
    base_dir().join("media").join("images")
}

pub fn media_audio() -> PathBuf {
    base_dir().join("media").join("audio")
}

pub fn tutors_dir() -> PathBuf {
    base_dir().join("tutors")
}

/// Create the media directories if they don't exist yet. On a fresh checkout
/// `media/` is absent (it's gitignored), so the first image/audio save would
/// otherwise fail silently. Called once at startup.
pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(media_images())?;
    std::fs::create_dir_all(media_audio())?;
    Ok(())
}
