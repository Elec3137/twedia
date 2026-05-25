use std::{ffi::OsStr, path::PathBuf};

/// returns the path with a filename that has `_edited` appended,
/// while still preserving the original extension.
pub async fn edited(mut path: PathBuf) -> PathBuf {
    let file_stem = path
        .file_stem()
        .unwrap_or_else(|| OsStr::new("media"))
        .to_string_lossy();

    let extension = path
        .extension()
        .unwrap_or_else(|| OsStr::new("mkv"))
        .to_string_lossy();

    path.set_file_name(format!("{file_stem}_edited.{extension}"));

    path
}
