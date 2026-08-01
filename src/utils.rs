use iced::Size;
use std::{
    ffi::OsStr,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};

pub trait BoolToggleExt {
    fn toggle(&mut self);
}

impl BoolToggleExt for bool {
    fn toggle(&mut self) {
        *self = !*self;
    }
}

pub trait SizeRatioExt<T> {
    fn get_aspect_ratio(&self) -> T;
}

impl<T> SizeRatioExt<T> for Size<T>
where
    T: std::ops::Div<Output = T> + Copy,
{
    fn get_aspect_ratio(&self) -> T {
        self.width / self.height
    }
}

/// takes the hash of a single slice and returns it
///
/// convinience function to avoid manually creating a Hasher,
/// only to discard it after using it once.
#[inline]
pub fn hash_chunk<T: Hash>(t: &[T]) -> u64 {
    let mut s = DefaultHasher::new();
    Hash::hash_slice(t, &mut s);
    s.finish()
}

pub trait PathBufExt {
    /// returns the path with a filename that has `_edited` appended,
    /// while still preserving the original extension.
    fn edited(self) -> PathBuf;
}

impl PathBufExt for PathBuf {
    fn edited(mut self) -> PathBuf {
        let file_stem = self
            .file_stem()
            .unwrap_or_else(|| OsStr::new("media"))
            .to_string_lossy();

        let extension = self
            .extension()
            .unwrap_or_else(|| OsStr::new("mkv"))
            .to_string_lossy();

        self.set_file_name(format!("{file_stem}_edited.{extension}"));

        self
    }
}
